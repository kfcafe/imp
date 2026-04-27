---
id: '271'
title: Add native YouTube video interpretation support to imp
slug: add-native-youtube-video-interpretation-support-to
status: open
priority: 2
created_at: '2026-04-27T03:32:51.695590Z'
updated_at: '2026-04-27T03:36:01.194766Z'
acceptance: A design and implementation plan exists in mana with clear scope, privacy/network tradeoffs, likely dependencies or external services, and concrete implementation units for the chosen first slice.
notes: |-
  ---
  2026-04-27T03:33:22.008073+00:00
  Initial inspection: imp's built-in web read path explicitly rejects YouTube URLs in `crates/imp-core/src/tools/web/read.rs` with `ReadError::YoutubeNotSupported`; the `web` tool then returns that message. Local environment has `yt-dlp 2026.02.21` and `ffmpeg 8.0.1`. For the sample URL, `yt-dlp --dump-json` can retrieve title/duration/channel/chapters and automatic captions availability, though it warns about YouTube n-challenge solving. This suggests a good first slice: add a YouTube-specific read path that uses metadata + subtitles/captions, preferably with no video download, with clear diagnostics/fallbacks when captions are unavailable or yt-dlp is missing. Avoid starting with raw video/audio multimodal analysis; that is a later fallback because it is heavier, slower, has privacy/network implications, and may require provider-specific media support.
labels:
- imp
- youtube
- context
- native-media
kind: epic
decisions:
- 'Use a pure Rust/HTTP extraction path as the first YouTube support direction instead of shelling out to yt-dlp. Rationale: imp needs granular control over video ingestion, diagnostics, transcript selection, attribution, privacy/network behavior, and later native context shaping. External CLI backends can remain optional fallbacks, not the primary architecture.'
---

Enable imp to treat YouTube URLs as first-class context sources rather than opaque web links. Desired outcome: given a YouTube URL such as https://www.youtube.com/watch?v=McO_xcf4IYw, imp can extract useful video context (metadata, transcript/captions when available, chapters if available, and source attribution) and make it available to the agent in a structured way. Initial design should prefer reliable transcript/metadata extraction before considering heavier audio/video downloading or multimodal model paths.
