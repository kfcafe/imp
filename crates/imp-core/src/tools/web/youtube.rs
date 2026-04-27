use reqwest::Client;
use serde_json::Value;
use url::Url;

use super::types::{ContentFormat, PageContent};

const WATCH_BASE_URL: &str = "https://www.youtube.com/watch";
const PLAYER_RESPONSE_VAR: &str = "ytInitialPlayerResponse";
const ANDROID_VR_CLIENT_NAME: &str = "28";
const ANDROID_VR_CLIENT_VERSION: &str = "1.71.26";
const ANDROID_VR_USER_AGENT: &str = "com.google.android.apps.youtube.vr.oculus/1.71.26 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip";

#[derive(Debug, Clone, PartialEq)]
struct VideoId(String);

impl VideoId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
struct CaptionTrack {
    base_url: String,
    language_code: String,
    name: String,
    is_generated: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct TranscriptSegment {
    start_ms: u64,
    duration_ms: Option<u64>,
    text: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct VideoMetadata {
    title: Option<String>,
    author: Option<String>,
    channel_id: Option<String>,
    duration_seconds: Option<String>,
    view_count: Option<String>,
    description: Option<String>,
    publish_date: Option<String>,
    upload_date: Option<String>,
}

struct CaptionSource {
    tracks: Vec<CaptionTrack>,
    selected_track: CaptionTrack,
    segments: Vec<TranscriptSegment>,
    source_client: &'static str,
}

pub async fn fetch_and_extract(client: &Client, url: &str) -> Result<PageContent, YouTubeError> {
    let parsed_url = Url::parse(url).map_err(|err| YouTubeError::InvalidUrl(err.to_string()))?;
    let video_id = extract_video_id(&parsed_url).ok_or(YouTubeError::UnsupportedUrl)?;
    let requested_url = url.to_string();
    let watch_url = canonical_watch_url(video_id.as_str());

    let response = client
        .get(watch_url.as_str())
        .header("User-Agent", super::read::USER_AGENT)
        .header("Accept", super::read::ACCEPT_HEADER)
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|err| YouTubeError::Fetch(err.to_string()))?;

    let status_code = response.status().as_u16();
    if !response.status().is_success() {
        return Err(YouTubeError::HttpStatus(
            status_code,
            response
                .status()
                .canonical_reason()
                .unwrap_or("Unknown")
                .to_string(),
        ));
    }

    let final_url = response.url().to_string();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let html = response
        .text()
        .await
        .map_err(|err| YouTubeError::Fetch(err.to_string()))?;
    let raw_body_bytes = html.len();

    let initial_player_response = extract_initial_player_response(&html)?;
    let metadata = extract_metadata(&initial_player_response);
    let visitor_data = extract_visitor_data(&html);
    let caption_source = resolve_caption_source(
        client,
        video_id.as_str(),
        &initial_player_response,
        visitor_data.as_deref(),
    )
    .await?;

    if caption_source.segments.is_empty() {
        return Err(YouTubeError::TranscriptEmpty);
    }

    let text = format_video_context(
        video_id.as_str(),
        &watch_url,
        &metadata,
        &caption_source.selected_track,
        &caption_source.segments,
    );
    let title = metadata
        .title
        .clone()
        .unwrap_or_else(|| format!("YouTube video {}", video_id.as_str()));
    let diagnostics = build_diagnostics(
        &caption_source.tracks,
        &caption_source.selected_track,
        caption_source.segments.len(),
        caption_source.source_client,
    );

    let was_redirected = final_url != watch_url;

    Ok(PageContent {
        title: Some(title),
        content_length: text.len(),
        text,
        url: final_url,
        requested_url,
        status_code,
        content_type,
        format_received: ContentFormat::Html,
        was_redirected,
        raw_body_bytes,
        diagnostics,
    })
}

async fn resolve_caption_source(
    client: &Client,
    video_id: &str,
    initial_player_response: &Value,
    visitor_data: Option<&str>,
) -> Result<CaptionSource, YouTubeError> {
    let web_result = fetch_caption_source_from_response(
        client,
        initial_player_response,
        super::read::USER_AGENT,
        "web",
    )
    .await;
    if web_result.is_ok() {
        return web_result;
    }
    let web_error = web_result.err();

    let Some(visitor_data) = visitor_data else {
        return Err(web_error.unwrap_or(YouTubeError::VisitorDataMissing));
    };

    let android_vr_response =
        fetch_android_vr_player_response(client, video_id, visitor_data).await?;
    fetch_caption_source_from_response(
        client,
        &android_vr_response,
        ANDROID_VR_USER_AGENT,
        "android_vr",
    )
    .await
}

async fn fetch_caption_source_from_response(
    client: &Client,
    player_response: &Value,
    user_agent: &str,
    source_client: &'static str,
) -> Result<CaptionSource, YouTubeError> {
    let tracks = extract_caption_tracks(player_response)?;
    let selected_track = select_caption_track(&tracks).ok_or(YouTubeError::NoUsableCaptionTrack)?;
    let segments = fetch_transcript_segments(client, &selected_track, user_agent).await?;

    Ok(CaptionSource {
        tracks,
        selected_track,
        segments,
        source_client,
    })
}

async fn fetch_transcript_segments(
    client: &Client,
    track: &CaptionTrack,
    user_agent: &str,
) -> Result<Vec<TranscriptSegment>, YouTubeError> {
    let transcript_url = caption_url_with_json3(&track.base_url)?;
    let transcript_response = client
        .get(transcript_url.as_str())
        .header("User-Agent", user_agent)
        .header(
            "Accept",
            "application/json,text/xml,text/plain;q=0.9,*/*;q=0.5",
        )
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|err| YouTubeError::Fetch(err.to_string()))?;

    if !transcript_response.status().is_success() {
        return Err(YouTubeError::TranscriptHttpStatus(
            transcript_response.status().as_u16(),
        ));
    }

    let transcript_text = transcript_response
        .text()
        .await
        .map_err(|err| YouTubeError::TranscriptParse(err.to_string()))?;
    if transcript_text.trim().is_empty() {
        return Err(YouTubeError::TranscriptEmpty);
    }

    if transcript_text.trim_start().starts_with('<') {
        return Ok(parse_xml_transcript(&transcript_text));
    }

    let transcript_json: Value = serde_json::from_str(&transcript_text)
        .map_err(|err| YouTubeError::TranscriptParse(err.to_string()))?;
    Ok(parse_json3_transcript(&transcript_json))
}

async fn fetch_android_vr_player_response(
    client: &Client,
    video_id: &str,
    visitor_data: &str,
) -> Result<Value, YouTubeError> {
    let payload = serde_json::json!({
        "context": {
            "client": {
                "clientName": "ANDROID_VR",
                "clientVersion": ANDROID_VR_CLIENT_VERSION,
                "deviceMake": "Oculus",
                "deviceModel": "Quest 3",
                "androidSdkVersion": 32,
                "userAgent": ANDROID_VR_USER_AGENT,
                "osName": "Android",
                "osVersion": "12L",
                "hl": "en",
                "gl": "US"
            }
        },
        "videoId": video_id,
        "contentCheckOk": true,
        "racyCheckOk": true
    });

    let response = client
        .post("https://www.youtube.com/youtubei/v1/player")
        .header("Content-Type", "application/json")
        .header("User-Agent", ANDROID_VR_USER_AGENT)
        .header("X-YouTube-Client-Name", ANDROID_VR_CLIENT_NAME)
        .header("X-YouTube-Client-Version", ANDROID_VR_CLIENT_VERSION)
        .header("X-Goog-Visitor-Id", visitor_data)
        .header("Origin", "https://www.youtube.com")
        .json(&payload)
        .send()
        .await
        .map_err(|err| YouTubeError::Fetch(err.to_string()))?;

    if !response.status().is_success() {
        return Err(YouTubeError::PlayerApiHttpStatus(
            response.status().as_u16(),
        ));
    }

    let player_response = response
        .json::<Value>()
        .await
        .map_err(|err| YouTubeError::PlayerResponseParse(err.to_string()))?;

    if player_response
        .pointer("/playabilityStatus/status")
        .and_then(Value::as_str)
        == Some("LOGIN_REQUIRED")
    {
        return Err(YouTubeError::PlayerApiLoginRequired(
            player_response
                .pointer("/playabilityStatus/reason")
                .and_then(Value::as_str)
                .unwrap_or("sign-in required")
                .to_string(),
        ));
    }

    Ok(player_response)
}

pub fn is_youtube_url(url: &Url) -> bool {
    url.host_str().is_some_and(|host| {
        let host = host.to_ascii_lowercase();
        host == "youtu.be"
            || host.ends_with(".youtu.be")
            || host == "youtube.com"
            || host.ends_with(".youtube.com")
    })
}

fn extract_video_id(url: &Url) -> Option<VideoId> {
    let host = url.host_str()?.to_ascii_lowercase();
    if host == "youtu.be" || host.ends_with(".youtu.be") {
        return first_path_segment(url).and_then(VideoId::from_candidate);
    }

    if !(host == "youtube.com" || host.ends_with(".youtube.com")) {
        return None;
    }

    match first_path_segment(url).as_deref() {
        Some("watch") => url
            .query_pairs()
            .find_map(|(key, value)| (key == "v").then(|| value.into_owned()))
            .and_then(VideoId::from_candidate),
        Some("shorts" | "embed" | "live") => url
            .path_segments()
            .and_then(|mut segments| segments.nth(1).map(str::to_string))
            .and_then(VideoId::from_candidate),
        _ => None,
    }
}

impl VideoId {
    fn from_candidate(candidate: String) -> Option<Self> {
        let id = candidate.trim();
        let is_valid = id.len() == 11
            && id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-');
        is_valid.then(|| Self(id.to_string()))
    }
}

fn first_path_segment(url: &Url) -> Option<String> {
    url.path_segments()
        .and_then(|mut segments| segments.next().map(str::to_string))
}

fn canonical_watch_url(video_id: &str) -> String {
    format!("{WATCH_BASE_URL}?v={video_id}")
}

fn extract_initial_player_response(html: &str) -> Result<Value, YouTubeError> {
    let marker_index = html
        .find(PLAYER_RESPONSE_VAR)
        .ok_or(YouTubeError::PlayerResponseMissing)?;
    let after_marker = &html[marker_index + PLAYER_RESPONSE_VAR.len()..];
    let brace_relative = after_marker
        .find('{')
        .ok_or(YouTubeError::PlayerResponseMissing)?;
    let json_start = marker_index + PLAYER_RESPONSE_VAR.len() + brace_relative;
    let json_end = find_balanced_json_end(html, json_start)?;
    serde_json::from_str(&html[json_start..json_end])
        .map_err(|err| YouTubeError::PlayerResponseParse(err.to_string()))
}

fn find_balanced_json_end(text: &str, start: usize) -> Result<usize, YouTubeError> {
    let mut depth = 0u32;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in text[start..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = in_string;
            continue;
        }

        if ch == '"' {
            in_string = !in_string;
            continue;
        }

        if in_string {
            continue;
        }

        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(start + offset + ch.len_utf8());
                }
            }
            _ => {}
        }
    }

    Err(YouTubeError::PlayerResponseUnterminated)
}

fn extract_visitor_data(html: &str) -> Option<String> {
    extract_quoted_json_field(html, "VISITOR_DATA")
        .or_else(|| extract_quoted_json_field(html, "visitorData"))
}

fn extract_quoted_json_field(text: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\":\"");
    let start = text.find(&marker)? + marker.len();
    let tail = &text[start..];
    let end = tail.find('"')?;
    Some(tail[..end].to_string())
}

fn extract_metadata(player_response: &Value) -> VideoMetadata {
    let details = player_response.get("videoDetails").unwrap_or(&Value::Null);
    let microformat = player_response
        .pointer("/microformat/playerMicroformatRenderer")
        .unwrap_or(&Value::Null);

    VideoMetadata {
        title: string_at(details, "title").or_else(|| text_runs_at(microformat, "title")),
        author: string_at(details, "author").or_else(|| string_at(microformat, "ownerChannelName")),
        channel_id: string_at(details, "channelId")
            .or_else(|| string_at(microformat, "externalChannelId")),
        duration_seconds: string_at(details, "lengthSeconds")
            .or_else(|| string_at(microformat, "lengthSeconds")),
        view_count: string_at(details, "viewCount").or_else(|| string_at(microformat, "viewCount")),
        description: string_at(details, "shortDescription")
            .or_else(|| text_runs_at(microformat, "description")),
        publish_date: string_at(microformat, "publishDate"),
        upload_date: string_at(microformat, "uploadDate"),
    }
}

fn extract_caption_tracks(player_response: &Value) -> Result<Vec<CaptionTrack>, YouTubeError> {
    let tracks = player_response
        .pointer("/captions/playerCaptionsTracklistRenderer/captionTracks")
        .and_then(Value::as_array)
        .ok_or(YouTubeError::CaptionTracksMissing)?;

    let parsed = tracks
        .iter()
        .filter_map(|track| {
            Some(CaptionTrack {
                base_url: string_at(track, "baseUrl")?,
                language_code: string_at(track, "languageCode")?,
                name: text_runs_at(track, "name").unwrap_or_else(|| "unknown".to_string()),
                is_generated: string_at(track, "kind").as_deref() == Some("asr"),
            })
        })
        .collect::<Vec<_>>();

    if parsed.is_empty() {
        return Err(YouTubeError::NoUsableCaptionTrack);
    }

    Ok(parsed)
}

fn select_caption_track(tracks: &[CaptionTrack]) -> Option<CaptionTrack> {
    tracks
        .iter()
        .find(|track| is_english(&track.language_code) && !track.is_generated)
        .or_else(|| tracks.iter().find(|track| is_english(&track.language_code)))
        .or_else(|| tracks.first())
        .cloned()
}

fn is_english(language_code: &str) -> bool {
    let language = language_code.to_ascii_lowercase();
    language == "en" || language.starts_with("en-") || language == "en-orig"
}

fn caption_url_with_json3(base_url: &str) -> Result<String, YouTubeError> {
    let mut url =
        Url::parse(base_url).map_err(|err| YouTubeError::InvalidCaptionUrl(err.to_string()))?;
    {
        let has_fmt = url.query_pairs().any(|(key, _)| key == "fmt");
        if !has_fmt {
            url.query_pairs_mut().append_pair("fmt", "json3");
        }
    }
    Ok(url.to_string())
}

fn parse_json3_transcript(value: &Value) -> Vec<TranscriptSegment> {
    value
        .get("events")
        .or_else(|| value.get("aAppend"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(parse_json3_event)
        .collect()
}

fn parse_json3_event(event: &Value) -> Option<TranscriptSegment> {
    let text = event
        .get("segs")?
        .as_array()?
        .iter()
        .filter_map(|seg| seg.get("utf8").and_then(Value::as_str))
        .collect::<String>();
    let text = normalize_transcript_text(&text);
    if text.is_empty() || is_noise_segment(&text) {
        return None;
    }

    Some(TranscriptSegment {
        start_ms: event.get("tStartMs").and_then(Value::as_u64).unwrap_or(0),
        duration_ms: event.get("dDurationMs").and_then(Value::as_u64),
        text,
    })
}

fn parse_xml_transcript(text: &str) -> Vec<TranscriptSegment> {
    text.split("<p ")
        .skip(1)
        .filter_map(parse_xml_paragraph)
        .collect()
}

fn parse_xml_paragraph(fragment: &str) -> Option<TranscriptSegment> {
    let tag_end = fragment.find('>')?;
    let attrs = &fragment[..tag_end];
    let body = &fragment[tag_end + 1..fragment.find("</p>")?];
    let text = normalize_transcript_text(&strip_xml_tags(body));
    if text.is_empty() || is_noise_segment(&text) {
        return None;
    }

    Some(TranscriptSegment {
        start_ms: extract_xml_time_ms(attrs, "t").unwrap_or(0),
        duration_ms: extract_xml_time_ms(attrs, "d"),
        text,
    })
}

fn extract_xml_time_ms(attrs: &str, key: &str) -> Option<u64> {
    let marker = format!(r#"{key}=""#);
    let start = attrs.find(&marker)? + marker.len();
    let tail = &attrs[start..];
    let end = tail.find('"')?;
    tail[..end].parse().ok()
}

fn strip_xml_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_tag = false;
    let mut entity = String::new();
    let mut in_entity = false;

    for ch in text.chars() {
        if in_entity {
            entity.push(ch);
            if ch == ';' {
                out.push_str(match entity.as_str() {
                    "amp;" => "&",
                    "lt;" => "<",
                    "gt;" => ">",
                    "quot;" => "\"",
                    "apos;" | "#39;" => "'",
                    _ => "",
                });
                entity.clear();
                in_entity = false;
            }
            continue;
        }

        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            '&' if !in_tag => in_entity = true,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }

    out
}

fn normalize_transcript_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_noise_segment(text: &str) -> bool {
    let normalized = text.trim().to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "[music]" | "[applause]" | "[laughter]" | "♪" | "♫"
    )
}

fn format_video_context(
    video_id: &str,
    canonical_url: &str,
    metadata: &VideoMetadata,
    track: &CaptionTrack,
    segments: &[TranscriptSegment],
) -> String {
    let mut output = String::new();
    output.push_str("# YouTube Video Context\n\n");
    output.push_str("## Source\n");
    output.push_str(&format!("- URL: {canonical_url}\n"));
    output.push_str(&format!("- Video ID: {video_id}\n"));
    push_optional(&mut output, "- Title", metadata.title.as_deref());
    push_optional(&mut output, "- Channel", metadata.author.as_deref());
    push_optional(&mut output, "- Channel ID", metadata.channel_id.as_deref());
    push_optional(
        &mut output,
        "- Duration seconds",
        metadata.duration_seconds.as_deref(),
    );
    push_optional(&mut output, "- Views", metadata.view_count.as_deref());
    push_optional(&mut output, "- Published", metadata.publish_date.as_deref());
    push_optional(&mut output, "- Uploaded", metadata.upload_date.as_deref());

    output.push_str("\n## Transcript Track\n");
    output.push_str(&format!("- Language: {}\n", track.language_code));
    output.push_str(&format!("- Name: {}\n", track.name));
    output.push_str(&format!("- Auto-generated: {}\n", track.is_generated));

    if let Some(description) = metadata.description.as_deref() {
        output.push_str("\n## Description\n");
        output.push_str(description.trim());
        output.push('\n');
    }

    output.push_str("\n## Transcript\n");
    for segment in segments {
        output.push_str(&format!(
            "[{}] {}\n",
            format_timestamp(segment.start_ms),
            segment.text
        ));
    }

    output.trim().to_string()
}

fn build_diagnostics(
    tracks: &[CaptionTrack],
    selected_track: &CaptionTrack,
    segment_count: usize,
    source_client: &str,
) -> Vec<String> {
    vec![
        "YouTube extraction used native HTTP transcript path; no video/audio was downloaded."
            .to_string(),
        format!(
            "Selected caption track from {source_client}: {} ({}, auto-generated: {}).",
            selected_track.name, selected_track.language_code, selected_track.is_generated
        ),
        format!(
            "Found {} caption track(s), extracted {} transcript segment(s).",
            tracks.len(),
            segment_count
        ),
    ]
}

fn push_optional(output: &mut String, label: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        output.push_str(&format!("{label}: {}\n", value.trim()));
    }
}

fn format_timestamp(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn string_at(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn text_runs_at(value: &Value, key: &str) -> Option<String> {
    let text_value = value.get(key)?;
    if let Some(simple_text) = text_value.get("simpleText").and_then(Value::as_str) {
        return Some(simple_text.to_string());
    }
    let runs = text_value.get("runs")?.as_array()?;
    let text = runs
        .iter()
        .filter_map(|run| run.get("text").and_then(Value::as_str))
        .collect::<String>();
    (!text.is_empty()).then_some(text)
}

#[derive(Debug)]
pub enum YouTubeError {
    InvalidUrl(String),
    UnsupportedUrl,
    Fetch(String),
    HttpStatus(u16, String),
    PlayerResponseMissing,
    PlayerResponseUnterminated,
    PlayerResponseParse(String),
    PlayerApiHttpStatus(u16),
    PlayerApiLoginRequired(String),
    VisitorDataMissing,
    CaptionTracksMissing,
    NoUsableCaptionTrack,
    InvalidCaptionUrl(String),
    TranscriptHttpStatus(u16),
    TranscriptParse(String),
    TranscriptEmpty,
}

impl std::fmt::Display for YouTubeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUrl(msg) => write!(f, "Invalid YouTube URL: {msg}"),
            Self::UnsupportedUrl => write!(f, "Unsupported YouTube URL"),
            Self::Fetch(msg) => write!(f, "YouTube fetch failed: {msg}"),
            Self::HttpStatus(code, reason) => write!(f, "YouTube returned HTTP {code} {reason}"),
            Self::PlayerResponseMissing => write!(f, "YouTube player response not found"),
            Self::PlayerResponseUnterminated => {
                write!(f, "YouTube player response JSON was unterminated")
            }
            Self::PlayerResponseParse(msg) => {
                write!(f, "YouTube player response parse failed: {msg}")
            }
            Self::PlayerApiHttpStatus(code) => write!(f, "YouTube player API returned HTTP {code}"),
            Self::PlayerApiLoginRequired(reason) => {
                write!(f, "YouTube player API required login: {reason}")
            }
            Self::VisitorDataMissing => write!(f, "YouTube visitor data was unavailable"),
            Self::CaptionTracksMissing => write!(f, "YouTube captions are unavailable for this video"),
            Self::NoUsableCaptionTrack => write!(f, "No usable YouTube caption track found"),
            Self::InvalidCaptionUrl(msg) => write!(f, "Invalid YouTube caption URL: {msg}"),
            Self::TranscriptHttpStatus(code) => write!(f, "YouTube transcript returned HTTP {code}"),
            Self::TranscriptParse(msg) => write!(f, "YouTube transcript parse failed: {msg}"),
            Self::TranscriptEmpty => write!(f, "YouTube transcript was empty; caption track metadata was found but YouTube returned no caption body for this client"),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn youtube_extracts_video_ids_from_common_urls() {
        let cases = [
            ("https://www.youtube.com/watch?v=McO_xcf4IYw", "McO_xcf4IYw"),
            ("https://youtu.be/McO_xcf4IYw?t=12", "McO_xcf4IYw"),
            ("https://www.youtube.com/shorts/McO_xcf4IYw", "McO_xcf4IYw"),
            ("https://www.youtube.com/embed/McO_xcf4IYw", "McO_xcf4IYw"),
        ];

        for (url, expected) in cases {
            let parsed = Url::parse(url).unwrap();
            assert_eq!(extract_video_id(&parsed).unwrap().as_str(), expected);
        }
    }

    #[test]
    fn youtube_rejects_invalid_video_ids() {
        let parsed =
            Url::parse("https://www.youtube.com/watch?v=not-valid-because-too-long").unwrap();
        assert!(extract_video_id(&parsed).is_none());
    }

    #[test]
    fn youtube_extracts_balanced_player_response() {
        let html = r#"<script>var ytInitialPlayerResponse = {"videoDetails":{"title":"A } in string","shortDescription":"escaped \" brace }"},"captions":{}};</script>"#;
        let response = extract_initial_player_response(html).unwrap();
        assert_eq!(response["videoDetails"]["title"], "A } in string");
        assert_eq!(
            response["videoDetails"]["shortDescription"],
            "escaped \" brace }"
        );
    }

    #[test]
    fn youtube_extracts_visitor_data() {
        let html = r#"ytcfg.set({"VISITOR_DATA":"visitor-token","other":true});"#;
        assert_eq!(extract_visitor_data(html).as_deref(), Some("visitor-token"));
    }

    #[test]
    fn youtube_selects_manual_english_before_auto_english() {
        let tracks = vec![
            CaptionTrack {
                base_url: "https://example.com/fr".into(),
                language_code: "fr".into(),
                name: "French".into(),
                is_generated: false,
            },
            CaptionTrack {
                base_url: "https://example.com/en-auto".into(),
                language_code: "en".into(),
                name: "English auto".into(),
                is_generated: true,
            },
            CaptionTrack {
                base_url: "https://example.com/en".into(),
                language_code: "en".into(),
                name: "English".into(),
                is_generated: false,
            },
        ];

        let selected = select_caption_track(&tracks).unwrap();
        assert_eq!(selected.base_url, "https://example.com/en");
    }

    #[test]
    fn youtube_parses_json3_transcript_segments() {
        let transcript = json!({
            "events": [
                {"tStartMs": 0, "dDurationMs": 1000, "segs": [{"utf8": "Hello "}, {"utf8": "world"}]},
                {"tStartMs": 1000, "segs": [{"utf8": "\n"}]},
                {"tStartMs": 2000, "segs": [{"utf8": "[Music]"}]},
                {"tStartMs": 3000, "dDurationMs": 500, "segs": [{"utf8": "next line"}]}
            ]
        });

        let segments = parse_json3_transcript(&transcript);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "Hello world");
        assert_eq!(segments[0].start_ms, 0);
        assert_eq!(segments[1].text, "next line");
    }

    #[test]
    fn youtube_parses_xml_transcript_segments() {
        let transcript = r#"<?xml version="1.0" ?><timedtext><body><p t="1000" d="2000">Hello &amp; <s>world</s></p><p t="3000" d="1000">[Music]</p></body></timedtext>"#;
        let segments = parse_xml_transcript(transcript);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].start_ms, 1000);
        assert_eq!(segments[0].duration_ms, Some(2000));
        assert_eq!(segments[0].text, "Hello & world");
    }

    #[test]
    fn youtube_adds_json3_format_to_caption_url() {
        let url =
            caption_url_with_json3("https://www.youtube.com/api/timedtext?v=abc&lang=en").unwrap();
        assert!(url.contains("fmt=json3"));
    }

    #[tokio::test]
    #[ignore = "network smoke test for YouTube extraction"]
    async fn youtube_reads_sample_video_over_http() {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .unwrap();
        let page = fetch_and_extract(&client, "https://www.youtube.com/watch?v=McO_xcf4IYw")
            .await
            .unwrap();
        assert!(page.text.contains("How to Build & Sell AI Services"));
        assert!(page.text.contains("## Transcript"));
        assert!(page.content_length > 1000);
    }
}
