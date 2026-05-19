# Example TypeScript Extension

This package demonstrates imp's manifest-first TypeScript extension bridge.

It intentionally uses only Node built-ins so it can run without installing npm
dependencies. The manifest declares two tools:

- `example_echo` — read-only demo tool that echoes input text, can return an
  error when `fail: true`, and includes metadata in the tool result.
- `example_write_demo` — workspace-write demo tool restricted by manifest policy
  to `tmp/example-extension/**`. This exists to demonstrate policy-gated side
  effects; use it only in throwaway workspaces.

Run the package self-test:

```sh
npm test --prefix extensions/example
```

The Rust test suite discovers the package by copying/linking it under
`.imp/extensions/example` and then invoking the manifest-backed tool path.
