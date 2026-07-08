import assert from "node:assert/strict";
import { test } from "node:test";
import type { AgentMessageRequest, HealthStatus, Session } from "./generated/types.js";
import { AuthenticationError, HttpTransport, SimError } from "./http-transport.js";

test("HttpTransport sends JSON requests and parses responses", async () => {
  const requests: Array<{ url: string; init?: RequestInit }> = [];
  const transport = new HttpTransport({
    baseUrl: "http://localhost:3030/",
    apiKey: "secret",
    fetch: async (input, init) => {
      requests.push({ url: input.toString(), init });
      assert.equal(init?.headers && (init.headers as Record<string, string>).authorization, "Bearer secret");
      return jsonResponse<HealthStatus>({ ok: true });
    },
  });

  assert.deepEqual(await transport.getHealth(), { ok: true });
  assert.equal(requests[0]?.url, "http://localhost:3030/health");
  assert.equal(requests[0]?.init?.method, "GET");
});

test("HttpTransport serializes message requests", async () => {
  let body: AgentMessageRequest | undefined;
  const transport = new HttpTransport({
    baseUrl: "http://localhost:3030",
    fetch: async (_input, init) => {
      body = JSON.parse(init?.body?.toString() ?? "{}") as AgentMessageRequest;
      return jsonResponse({
        session: sessionFixture(),
        message: { role: "assistant", content: "done" },
        output: "done",
      });
    },
  });

  const response = await transport.sendMessage({
    session_id: "session-1",
    message: { role: "user", content: "hello" },
    stream: false,
  });

  assert.equal(body?.session_id, "session-1");
  assert.equal(response.output, "done");
});

test("HttpTransport converts API failures into typed errors", async () => {
  const transport = new HttpTransport({
    baseUrl: "http://localhost:3030",
    fetch: async () =>
      new Response(JSON.stringify({ code: "unauthorized", message: "bad token" }), {
        status: 401,
        headers: { "content-type": "application/json" },
      }),
  });

  await assert.rejects(() => transport.getHealth(), (error) => {
    assert.ok(error instanceof AuthenticationError);
    assert.equal(error.message, "bad token");
    assert.equal(error.status, 401);
    return true;
  });
});

test("HttpTransport rejects invalid configuration", () => {
  assert.throws(() => new HttpTransport({ baseUrl: "" }), SimError);
});

function jsonResponse<T>(body: T): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
}

function sessionFixture(): Session {
  return {
    id: "session-1",
    title: "Mock session",
    created_at: "2026-07-08T00:00:00Z",
    updated_at: "2026-07-08T00:00:00Z",
    message_count: 1,
    status: "active",
  };
}
