# HN docs updates

Updated HN-facing docs to close the audited gaps:

- `README.md`: split install and quickstart so first-time visitors get a clearer path.
- `README.md`: added source-install fallback with `cargo install --path .` and `imp install-local` for readers trying the current checkout.
- `README.md`: clarified ACP as a scaffold/internal 0.3.0 surface instead of a general planned editor adapter claim.
- `README.md`: removed stale legacy mana compatibility bullet from the HN-facing status section.
- `README.md`: linked the ACP scaffold doc from technical docs with an explicit out-of-scope note.
- `docs/index.md`: added the ACP scaffold doc to the core reference index with current limitations called out.

Verification:

- Reviewed `git diff -- README.md .imp/workflows/docs-hn-readiness`.
- Ran `git diff --check -- README.md .imp/workflows/docs-hn-readiness` successfully.
