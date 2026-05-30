# ACP smoke-client demo

Current scaffold smoke command:

```sh
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{},"clientInfo":{"name":"smoke"}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":"/Users/asher/imp","mcpServers":[]}}' \
  | cargo run -q -p imp-cli -- acp
```

Expected transcript shape:

```jsonl
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"agentCapabilities":{"promptCapabilities":{"embeddedContext":true},"sessionCapabilities":{"close":{}}},"agentInfo":{"name":"imp","title":"imp","version":"0.3.0"},"authMethods":[]}}
{"jsonrpc":"2.0","id":2,"result":{"sessionId":"<imp-session-uuid>"}}
```

Focused automated coverage currently exercises:

- initialize handshake;
- session/new and durable imp session creation;
- session/prompt scaffold completion shape;
- session/cancel scaffold state;
- event-to-session/update mapping.

Verified command:

```sh
cargo test -p imp-cli acp -- --nocapture
```

Result: 18 ACP tests passed.

Concern: this is not yet a live editor demo. `session/prompt` currently returns a scaffolded `end_turn` completion instead of running the provider-backed imp agent loop.
