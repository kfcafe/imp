# Results

- Text-based stop detection was removed for normal execution. Assistant prose such as `done`, `blocked`, or `before continuing` no longer maps to runtime text fallback stop reasons.
- The generated system prompt no longer includes the hard-coded `Operating rules:` behavioral instruction block. Tool routing and factual tool availability remain.
- Runtime-policy implementation was committed as `c0b7d97 Simplify agent runtime stop policy`.
- Verification passed through the workflow:
  - `cargo test -p imp-core text_stop --lib`
  - `cargo test -p imp-core system_prompt --lib`
  - `cargo test -p imp-core agent:: --lib`
  - `cargo test -p imp-core --lib`

No remaining runtime-policy concerns for this workflow.
