use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};

/// PKCE (Proof Key for Code Exchange) challenge pair for OAuth 2.0.
pub struct PkceChallenge {
    /// The code verifier — a high-entropy random string sent during token exchange.
    pub verifier: String,
    /// The code challenge — a SHA-256 hash of the verifier, sent in the authorization request.
    pub challenge: String,
}

impl PkceChallenge {
    /// Generate a new PKCE verifier/challenge pair.
    ///
    /// Uses 32 cryptographically random bytes, base64url-encoded (no padding).
    /// The challenge is the SHA-256 hash of the verifier, also base64url-encoded.
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);

        let verifier = URL_SAFE_NO_PAD.encode(bytes);
        let hash = Sha256::digest(verifier.as_bytes());
        let challenge = URL_SAFE_NO_PAD.encode(hash);

        Self {
            verifier,
            challenge,
        }
    }

    /// Verify that a challenge matches the SHA-256 of the given verifier.
    pub fn verify(verifier: &str, challenge: &str) -> bool {
        let hash = Sha256::digest(verifier.as_bytes());
        let expected = URL_SAFE_NO_PAD.encode(hash);
        expected == challenge
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_pkce_verifier_length() {
        let pkce = PkceChallenge::generate();
        // 32 bytes → 43 base64url chars (no padding)
        assert_eq!(pkce.verifier.len(), 43);
    }

    #[test]
    fn test_oauth_pkce_challenge_matches_verifier() {
        let pkce = PkceChallenge::generate();
        assert!(PkceChallenge::verify(&pkce.verifier, &pkce.challenge));
    }

    #[test]
    fn test_oauth_pkce_unique() {
        let a = PkceChallenge::generate();
        let b = PkceChallenge::generate();
        assert_ne!(a.verifier, b.verifier);
        assert_ne!(a.challenge, b.challenge);
    }

    #[test]
    fn test_oauth_pkce_challenge_is_base64url() {
        let pkce = PkceChallenge::generate();
        // Should only contain base64url characters
        assert!(pkce
            .challenge
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert!(pkce
            .verifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn test_oauth_pkce_wrong_verifier_fails() {
        let pkce = PkceChallenge::generate();
        assert!(!PkceChallenge::verify("wrong-verifier", &pkce.challenge));
    }
}
