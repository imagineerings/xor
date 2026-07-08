import assert from "node:assert/strict";
import { test } from "node:test";
import type {
  AgentMessageRequest,
  ClientCapabilities,
  GooseClientConfig,
  Message,
  StreamEvent,
} from "./generated/types.js";

test("generated SDK types accept representative client payloads", () => {
  const config: GooseClientConfig = {
    mode: "http",
    baseUrl: "http://localhost:3030",
    capabilities: {
      platform: "node",
      features: ["streaming", "mcp_apps"],
    },
  };
  const message: Message = {
    role: "user",
    content: [{ type: "text", text: "hello" }],
  };
  const request: AgentMessageRequest = {
    session_id: "session-1",
    message,
    stream: true,
  };
  const capabilities: ClientCapabilities = {
    version: "0.1.0",
    platform: "node",
    features: ["streaming", "recipes"],
    streaming: true,
    maxMessageSize: 1024,
  };
  const event: StreamEvent = { type: "done" };

  assert.equal(config.mode, "http");
  assert.equal(request.message.role, "user");
  assert.equal(capabilities.streaming, true);
  assert.equal(event.type, "done");
});
