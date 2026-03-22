use std::time::Duration;

use futures::StreamExt;
use imp_llm::{provider::RetryPolicy, StreamEvent};

/// Determines whether an `imp_llm::Error` is transient and worth retrying.
///
/// Non-retryable errors (auth failures, bad requests) should propagate
/// immediately — retrying them wastes time and may make things worse.
pub fn is_retryable(err: &imp_llm::Error) -> bool {
    match err {
        // Rate limit — always retry; the provider says to wait.
        imp_llm::Error::RateLimited { .. } => true,
        // HTTP transport failures: check what kind of reqwest error it is.
        imp_llm::Error::Http(e) => e.is_connect() || e.is_timeout() || e.is_request(),
        // Stream errors are transient (connection reset, partial read, etc.).
        imp_llm::Error::Stream(_) => true,
        // Provider errors may carry an HTTP status in the message. Check for 5xx.
        imp_llm::Error::Provider(msg) => {
            msg.contains("HTTP 500")
                || msg.contains("HTTP 502")
                || msg.contains("HTTP 503")
                || msg.contains("HTTP 529")
        }
        // Auth errors (401, 403) and bad request (400) are permanent.
        imp_llm::Error::Auth(_) => false,
        // Serialization, IO, context-too-long: not transient.
        imp_llm::Error::Serialization(_)
        | imp_llm::Error::Io(_)
        | imp_llm::Error::ContextTooLong { .. } => false,
    }
}

/// Compute how long to wait before a retry attempt.
///
/// Uses exponential backoff with random jitter in [0, base_delay / 2).
/// If the error carries a `Retry-After` hint that is within `max_delay`,
/// that takes precedence.
pub fn backoff_delay(
    attempt: u32,
    policy: &RetryPolicy,
    retry_after_secs: Option<u64>,
) -> Option<Duration> {
    // If the provider told us exactly when to retry, respect it — unless it
    // exceeds our maximum, in which case we give up immediately.
    if let Some(secs) = retry_after_secs {
        let suggested = Duration::from_secs(secs);
        if suggested > policy.max_delay {
            return None; // signal: abort, don't retry
        }
        return Some(suggested);
    }

    // Exponential backoff: base * 2^attempt, capped at max_delay.
    let base_ms = policy.base_delay.as_millis() as u64;
    let exp_ms = base_ms.saturating_mul(1u64 << attempt.min(10));
    let capped_ms = exp_ms.min(policy.max_delay.as_millis() as u64);

    // Jitter: add up to 50% of the capped delay to spread retries.
    let jitter_ms = rand::random::<u64>() % (capped_ms / 2 + 1);

    Some(Duration::from_millis(capped_ms + jitter_ms))
}

/// Run a streaming LLM call with automatic retry on transient errors.
///
/// `make_stream` is called once per attempt. If the stream yields an `Err`
/// that is retryable (and we have attempts left), the whole stream is
/// discarded and `make_stream` is called again after a backoff delay.
///
/// Transparent to the caller: it receives the same `Vec<StreamEvent>` items
/// that would have come from a successful first attempt.
///
/// Returns `Err` if:
/// - A non-retryable error is encountered
/// - `max_retries` is exhausted
/// - A `RateLimited` retry-after exceeds `max_delay`
pub async fn run_with_retry<F, S>(
    mut make_stream: F,
    policy: &RetryPolicy,
) -> imp_llm::Result<Vec<imp_llm::Result<StreamEvent>>>
where
    F: FnMut() -> S,
    S: futures_core::Stream<Item = imp_llm::Result<StreamEvent>>,
{
    let mut attempt = 0u32;
    loop {
        let mut stream = make_stream();
        let mut events: Vec<imp_llm::Result<StreamEvent>> = Vec::new();
        let mut stream_error: Option<imp_llm::Error> = None;

        while let Some(item) = stream.next().await {
            match item {
                Ok(ev) => events.push(Ok(ev)),
                Err(e) => {
                    stream_error = Some(e);
                    break;
                }
            }
        }

        match stream_error {
            None => {
                // Stream completed without error.
                return Ok(events);
            }
            Some(err) => {
                let retry_after = if let imp_llm::Error::RateLimited { retry_after_secs } = &err {
                    *retry_after_secs
                } else {
                    None
                };

                if !is_retryable(&err) || attempt >= policy.max_retries {
                    return Err(err);
                }

                match backoff_delay(attempt, policy, retry_after) {
                    None => {
                        // retry-after exceeds max_delay — abort.
                        return Err(err);
                    }
                    Some(delay) => {
                        tokio::time::sleep(delay).await;
                        attempt += 1;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use imp_llm::provider::RetryCondition;

    fn default_policy() -> RetryPolicy {
        RetryPolicy {
            max_retries: 3,
            base_delay: Duration::from_millis(10), // fast for tests
            max_delay: Duration::from_millis(100),
            retry_on: vec![
                RetryCondition::RateLimit,
                RetryCondition::ServerError,
                RetryCondition::Timeout,
                RetryCondition::ConnectionError,
            ],
        }
    }

    // ── is_retryable ──────────────────────────────────────────────

    #[test]
    fn rate_limited_is_retryable() {
        let err = imp_llm::Error::RateLimited {
            retry_after_secs: Some(5),
        };
        assert!(is_retryable(&err));
    }

    #[test]
    fn stream_error_is_retryable() {
        let err = imp_llm::Error::Stream("connection reset".into());
        assert!(is_retryable(&err));
    }

    #[test]
    fn auth_error_is_not_retryable() {
        let err = imp_llm::Error::Auth("invalid key".into());
        assert!(!is_retryable(&err));
    }

    #[test]
    fn provider_5xx_is_retryable() {
        let err = imp_llm::Error::Provider("HTTP 503: overloaded".into());
        assert!(is_retryable(&err));
    }

    #[test]
    fn provider_4xx_is_not_retryable() {
        let err = imp_llm::Error::Provider("HTTP 400: bad request".into());
        assert!(!is_retryable(&err));
    }

    #[test]
    fn provider_401_is_not_retryable() {
        let err = imp_llm::Error::Provider("HTTP 401: unauthorized".into());
        assert!(!is_retryable(&err));
    }

    // ── backoff_delay ─────────────────────────────────────────────

    #[test]
    fn backoff_grows_exponentially() {
        let policy = default_policy();
        let d0 = backoff_delay(0, &policy, None).unwrap();
        let d1 = backoff_delay(1, &policy, None).unwrap();
        let d2 = backoff_delay(2, &policy, None).unwrap();
        // Each step should generally be larger (accounting for jitter).
        // At minimum: base*2^0=10ms, base*2^1=20ms, base*2^2=40ms
        // With jitter added, d1 >= 20ms and d2 >= 40ms (before jitter on d0).
        // We can only assert upper bounds reliably given jitter.
        assert!(d0 <= Duration::from_millis(200)); // 10ms base + 50% jitter, capped
        assert!(d1 >= Duration::from_millis(20));
        assert!(d2 >= Duration::from_millis(40));
    }

    #[test]
    fn backoff_capped_at_max_delay() {
        let policy = default_policy(); // max 100ms
        // Attempt 10 would be base(10ms) * 2^10 = 10_240ms → capped at 100ms
        let delay = backoff_delay(10, &policy, None).unwrap();
        assert!(delay <= Duration::from_millis(200)); // cap + up to 50% jitter of cap
    }

    #[test]
    fn retry_after_respected_within_limit() {
        let policy = default_policy(); // max 100ms
        let delay = backoff_delay(0, &policy, Some(0)).unwrap();
        assert_eq!(delay, Duration::from_secs(0));
    }

    #[test]
    fn retry_after_exceeds_max_delay_returns_none() {
        let policy = default_policy(); // max 100ms
        let result = backoff_delay(0, &policy, Some(10)); // 10s > 100ms
        assert!(result.is_none());
    }

    // ── run_with_retry ────────────────────────────────────────────

    #[tokio::test]
    async fn retry_succeeds_after_transient_failures() {
        use std::sync::{Arc, Mutex};

        let call_count = Arc::new(Mutex::new(0u32));

        let policy = RetryPolicy {
            max_retries: 3,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(50),
            retry_on: vec![RetryCondition::ServerError],
        };

        let call_count_clone = call_count.clone();
        let result = run_with_retry(
            move || {
                let mut count = call_count_clone.lock().unwrap();
                *count += 1;
                let attempt = *count;
                drop(count);

                // Fail the first 2 calls with a retryable error, succeed on 3rd
                if attempt < 3 {
                    let events: Vec<imp_llm::Result<StreamEvent>> = vec![
                        Ok(StreamEvent::MessageStart {
                            model: "test".into(),
                        }),
                        Err(imp_llm::Error::Stream("transient".into())),
                    ];
                    futures::stream::iter(events)
                } else {
                    let events: Vec<imp_llm::Result<StreamEvent>> = vec![
                        Ok(StreamEvent::MessageStart {
                            model: "test".into(),
                        }),
                        Ok(StreamEvent::TextDelta { text: "hello".into() }),
                    ];
                    futures::stream::iter(events)
                }
            },
            &policy,
        )
        .await
        .unwrap();

        assert_eq!(*call_count.lock().unwrap(), 3);
        assert_eq!(result.len(), 2); // MessageStart + TextDelta from successful attempt
        assert!(matches!(
            result[0],
            Ok(StreamEvent::MessageStart { .. })
        ));
        assert!(matches!(
            result[1],
            Ok(StreamEvent::TextDelta { .. })
        ));
    }

    #[tokio::test]
    async fn retry_exhausts_max_retries() {
        use std::sync::{Arc, Mutex};

        let call_count = Arc::new(Mutex::new(0u32));

        let policy = RetryPolicy {
            max_retries: 2,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(50),
            retry_on: vec![RetryCondition::ServerError],
        };

        let call_count_clone = call_count.clone();
        let result = run_with_retry(
            move || {
                *call_count_clone.lock().unwrap() += 1;
                let events: Vec<imp_llm::Result<StreamEvent>> =
                    vec![Err(imp_llm::Error::Stream("always fails".into()))];
                futures::stream::iter(events)
            },
            &policy,
        )
        .await;

        // max_retries=2 means: 1 initial attempt + 2 retries = 3 total calls
        assert_eq!(*call_count.lock().unwrap(), 3);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn retry_skips_non_retryable_errors() {
        use std::sync::{Arc, Mutex};

        let call_count = Arc::new(Mutex::new(0u32));

        let policy = RetryPolicy {
            max_retries: 3,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(50),
            retry_on: vec![RetryCondition::ServerError],
        };

        let call_count_clone = call_count.clone();
        let result = run_with_retry(
            move || {
                *call_count_clone.lock().unwrap() += 1;
                // Auth errors are not retryable
                let events: Vec<imp_llm::Result<StreamEvent>> =
                    vec![Err(imp_llm::Error::Auth("invalid key".into()))];
                futures::stream::iter(events)
            },
            &policy,
        )
        .await;

        // Should fail immediately without retry
        assert_eq!(*call_count.lock().unwrap(), 1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), imp_llm::Error::Auth(_)));
    }

    #[tokio::test]
    async fn retry_no_error_passes_through() {
        let policy = default_policy();

        let result = run_with_retry(
            || {
                let events: Vec<imp_llm::Result<StreamEvent>> = vec![
                    Ok(StreamEvent::MessageStart {
                        model: "test".into(),
                    }),
                    Ok(StreamEvent::TextDelta { text: "ok".into() }),
                ];
                futures::stream::iter(events)
            },
            &policy,
        )
        .await
        .unwrap();

        assert_eq!(result.len(), 2);
    }
}
