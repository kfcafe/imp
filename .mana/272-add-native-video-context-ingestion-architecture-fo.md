---
id: '272'
title: Add native video context ingestion architecture for imp
slug: add-native-video-context-ingestion-architecture-fo
status: open
priority: 2
created_at: '2026-04-27T03:37:00.305621Z'
updated_at: '2026-04-27T03:45:37.110992Z'
notes: |-
  ---
  2026-04-27T03:45:37.110984+00:00
  Found `akinsella/yt-transcript-rs` (MIT, crate `yt-transcript-rs` v0.1.8). README says it fetches transcripts, lists transcript tracks, fetches video details/microformat/streaming data, supports proxies/cookie auth, and now uses YouTube InnerTube API exclusively due to YouTube API changes. Dependencies include reqwest, tokio, serde/serde_json, url, chrono, regex, quick-xml, html2text/html-escape, scraper, clap, anyhow/thiserror. Recommendation: use it as a reference implementation first, especially for InnerTube request shape, transcript list modeling, parsing, and error taxonomy. Do not add as an imp dependency by default without auditing dependency surface and API fit; it pulls in CLI/dev-ish deps and may expose more media extraction surface than imp needs. Consider vendoring/adapting the minimal InnerTube transcript logic or adding it behind a feature after audit.
labels:
- tower
- imp
- youtube
- video-context
- architecture
kind: epic
---

Cross-project/root planning container for native YouTube/video interpretation support across Tower. Current chosen direction: imp should implement YouTube ingestion through a pure Rust/HTTP path first, not yt-dlp or media download, so the runtime has granular control over metadata, transcript choice, diagnostics, attribution, privacy/network behavior, and future context shaping. Initial target remains imp's existing `web.read` seam, where YouTube URLs are currently rejected. Related project-local planning exists in `/Users/asher/imp/.mana` as epic 271 and child 271.1; keep root as source of truth for architecture/decomposition if the work expands beyond imp-core.
