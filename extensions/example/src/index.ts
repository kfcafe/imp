type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

type ToolCallRequest = {
  id?: string;
  type?: string;
  tool?: string;
  input?: Record<string, JsonValue>;
  context?: {
    cwd?: string;
    extensionId?: string;
    runId?: string;
    timeoutMs?: number;
  };
};

type ToolResponse = {
  id: string;
  type: "tool_result" | "error";
  content?: Array<{ type: "text"; text: string }>;
  details?: Record<string, JsonValue>;
  message?: string;
  code?: string;
  isError?: boolean;
};

async function readStdin(): Promise<string> {
  const chunks: Uint8Array[] = [];
  for await (const chunk of Bun.stdin.stream()) {
    chunks.push(chunk);
  }
  return new TextDecoder().decode(Buffer.concat(chunks));
}

function textInput(input: Record<string, JsonValue> | undefined, key: string): string {
  const value = input?.[key];
  return typeof value === "string" ? value : "";
}

function handle(request: ToolCallRequest): ToolResponse {
  const id = request.id ?? "call-1";
  switch (request.tool) {
    case "example_echo": {
      const text = textInput(request.input, "text");
      if (!text) {
        return {
          id,
          type: "error",
          message: "example_echo requires non-empty text",
          code: "invalid_input"
        };
      }
      return {
        id,
        type: "tool_result",
        content: [{ type: "text", text }],
        details: {
          extension: request.context?.extensionId ?? "dev.imp.example",
          runId: request.context?.runId ?? null,
          length: text.length
        }
      };
    }
    case "example_write_note": {
      return {
        id,
        type: "error",
        message:
          "example_write_note is declared as workspace-write and should be policy-gated by Rust before execution.",
        code: "policy_gated_demo"
      };
    }
    default:
      return {
        id,
        type: "error",
        message: `Unknown example extension tool: ${request.tool ?? "<missing>"}`,
        code: "unknown_tool"
      };
  }
}

const raw = await readStdin();
const request = JSON.parse(raw) as ToolCallRequest;
console.log(JSON.stringify(handle(request)));
