import assert from "node:assert/strict";
import { test } from "node:test";
import { HttpStreamClient, parseSseStream } from "./stream.js";

test("parseSseStream decodes message and done events", async () => {
  const stream = new ReadableStream<Uint8Array>({
    start(controller) {
      const encoder = new TextEncoder();
      controller.enqueue(
        encoder.encode(
          'event: message\ndata: {"type":"message","message":{"role":"assistant","content":"hi"}}\n\n',
        ),
      );
      controller.enqueue(encoder.encode('event: done\ndata: {"type":"done"}\n\n'));
      controller.close();
    },
  });

  const events = [];
  for await (const event of parseSseStream(stream)) {
    events.push(event);
  }

  assert.deepEqual(events, [
    { type: "message", message: { role: "assistant", content: "hi" } },
    { type: "done" },
  ]);
});

test("HttpStreamClient posts streaming message requests", async () => {
  let body: unknown;
  const client = new HttpStreamClient({
    baseUrl: "http://localhost:3030",
    fetch: async (_input, init) => {
      body = JSON.parse(init?.body?.toString() ?? "{}");
      return new Response('event: done\ndata: {"type":"done"}\n\n', {
        status: 200,
        headers: { "content-type": "text/event-stream" },
      });
    },
  });

  const events = [];
  for await (const event of client.streamMessage({
    session_id: "session-1",
    message: { role: "user", content: "hello" },
  })) {
    events.push(event);
  }

  assert.deepEqual(body, {
    session_id: "session-1",
    message: { role: "user", content: "hello" },
    stream: true,
  });
  assert.deepEqual(events, [{ type: "done" }]);
});
