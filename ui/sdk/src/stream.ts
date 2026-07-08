import type { AgentMessageRequest, StreamEvent } from "./generated/types.js";
import { AuthenticationError, SimError, TimeoutError, type FetchLike } from "./http-transport.js";

export interface HttpStreamOptions {
  baseUrl: string;
  apiKey?: string;
  fetch?: FetchLike;
  timeout?: number;
  maxRetries?: number;
  retryBackoff?: number;
}

export interface StreamOptions {
  signal?: AbortSignal;
  maxRetries?: number;
}

export type ReconnectCallback = (attempt: number) => void;

export class HttpStreamClient {
  readonly baseUrl: string;
  private readonly apiKey?: string;
  private readonly timeout: number;
  private readonly retryBackoff: number;
  private readonly fetchImpl: FetchLike;
  private maxRetries: number;
  private readonly reconnectCallbacks = new Set<ReconnectCallback>();

  constructor(options: HttpStreamOptions) {
    if (!options.baseUrl.trim()) {
      throw new SimError("baseUrl is required", { code: "invalid_config" });
    }

    this.baseUrl = options.baseUrl.replace(/\/+$/, "");
    this.apiKey = options.apiKey;
    this.timeout = options.timeout ?? 30_000;
    this.maxRetries = options.maxRetries ?? 3;
    this.retryBackoff = options.retryBackoff ?? 500;
    this.fetchImpl = options.fetch ?? globalThis.fetch?.bind(globalThis);

    if (!this.fetchImpl) {
      throw new SimError("fetch is not available; provide HttpStreamOptions.fetch", {
        code: "missing_fetch",
      });
    }
  }

  onReconnect(callback: ReconnectCallback): () => void {
    this.reconnectCallbacks.add(callback);
    return () => this.reconnectCallbacks.delete(callback);
  }

  setMaxRetries(retries: number): void {
    if (!Number.isInteger(retries) || retries < 0) {
      throw new SimError("max retries must be a non-negative integer", {
        code: "invalid_config",
      });
    }
    this.maxRetries = retries;
  }

  streamEvents(sessionId: string, options: StreamOptions = {}): AsyncGenerator<StreamEvent> {
    return this.withReconnect(
      `/sessions/${encodeURIComponent(sessionId)}/events`,
      { method: "GET" },
      options,
    );
  }

  streamMessage(
    request: AgentMessageRequest,
    options: StreamOptions = {},
  ): AsyncGenerator<StreamEvent> {
    return this.withReconnect(
      "/agent/stream",
      {
        method: "POST",
        body: JSON.stringify({ ...request, stream: true }),
      },
      options,
    );
  }

  private async *withReconnect(
    path: string,
    init: RequestInit,
    options: StreamOptions,
  ): AsyncGenerator<StreamEvent> {
    const maxRetries = options.maxRetries ?? this.maxRetries;
    for (let attempt = 0; ; attempt += 1) {
      try {
        yield* this.openStream(path, init, options.signal);
        return;
      } catch (error) {
        if (options.signal?.aborted) {
          throw error;
        }
        if (attempt >= maxRetries) {
          throw error;
        }
        this.notifyReconnect(attempt + 1);
        await delay(this.retryBackoff * 2 ** attempt, options.signal);
      }
    }
  }

  private async *openStream(
    path: string,
    init: RequestInit,
    signal?: AbortSignal,
  ): AsyncGenerator<StreamEvent> {
    const controller = new AbortController();
    const abort = () => controller.abort();
    signal?.addEventListener("abort", abort, { once: true });
    const timeout = setTimeout(() => controller.abort(), this.timeout);

    try {
      const response = await this.fetchImpl(this.url(path), {
        ...init,
        headers: this.headers(init.body),
        signal: controller.signal,
      });
      clearTimeout(timeout);
      await this.ensureStreamResponse(response);
      if (!response.body) {
        throw new SimError("stream response did not include a body", { code: "missing_body" });
      }

      for await (const event of parseSseStream(response.body)) {
        yield event;
        if (event.type === "done") {
          return;
        }
      }
    } catch (error) {
      if (isAbortError(error)) {
        throw new TimeoutError(`Stream request timed out after ${this.timeout}ms`);
      }
      if (error instanceof SimError) {
        throw error;
      }
      throw new SimError(error instanceof Error ? error.message : "stream request failed", {
        code: "stream_failed",
      });
    } finally {
      clearTimeout(timeout);
      signal?.removeEventListener("abort", abort);
    }
  }

  private async ensureStreamResponse(response: Response): Promise<void> {
    if (response.ok) {
      return;
    }

    const payload = await readErrorPayload(response);
    const message =
      typeof payload === "object" &&
      payload !== null &&
      "message" in payload &&
      typeof (payload as { message: unknown }).message === "string"
        ? (payload as { message: string }).message
        : response.statusText || `HTTP ${response.status}`;
    const options = {
      status: response.status,
      details: payload,
    };

    if (response.status === 401 || response.status === 403) {
      throw new AuthenticationError(message, options);
    }
    throw new SimError(message, options);
  }

  private url(path: string): string {
    if (/^https?:\/\//.test(path)) {
      return path;
    }
    return `${this.baseUrl}/${path.replace(/^\/+/, "")}`;
  }

  private headers(body: BodyInit | null | undefined): HeadersInit {
    const headers: Record<string, string> = {
      accept: "text/event-stream",
    };
    if (body !== undefined && body !== null) {
      headers["content-type"] = "application/json";
    }
    if (this.apiKey) {
      headers.authorization = `Bearer ${this.apiKey}`;
    }
    return headers;
  }

  private notifyReconnect(attempt: number): void {
    for (const callback of this.reconnectCallbacks) {
      callback(attempt);
    }
  }
}

export async function* parseSseStream(stream: ReadableStream<Uint8Array>): AsyncGenerator<StreamEvent> {
  const reader = stream.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) {
        break;
      }
      buffer += decoder.decode(value, { stream: true });
      yield* drainSseBuffer(buffer, (remaining) => {
        buffer = remaining;
      });
    }
    buffer += decoder.decode();
    if (buffer.trim()) {
      yield decodeSseEvent(buffer);
    }
  } finally {
    reader.releaseLock();
  }
}

function* drainSseBuffer(
  buffer: string,
  updateRemaining: (remaining: string) => void,
): Generator<StreamEvent> {
  let remaining = buffer;
  for (;;) {
    const separator = remaining.indexOf("\n\n");
    if (separator === -1) {
      updateRemaining(remaining);
      return;
    }
    const frame = remaining.slice(0, separator);
    remaining = remaining.slice(separator + 2);
    if (frame.trim()) {
      yield decodeSseEvent(frame);
    }
  }
}

function decodeSseEvent(frame: string): StreamEvent {
  let eventType = "message";
  const data = [];

  for (const line of frame.split(/\r?\n/)) {
    if (line.startsWith(":")) {
      continue;
    }
    const separator = line.indexOf(":");
    const field = separator === -1 ? line : line.slice(0, separator);
    const value = separator === -1 ? "" : line.slice(separator + 1).replace(/^ /, "");
    if (field === "event") {
      eventType = value;
    } else if (field === "data") {
      data.push(value);
    }
  }

  const payload = data.join("\n");
  if (eventType === "done" || payload === "[DONE]") {
    return { type: "done" };
  }
  if (eventType === "error") {
    return { type: "error", error: payload };
  }

  try {
    const parsed = JSON.parse(payload) as StreamEvent;
    if (isStreamEvent(parsed)) {
      return parsed;
    }
  } catch {
    // Fall through to a text message event.
  }

  return {
    type: "message",
    message: {
      role: "assistant",
      content: payload,
    },
  };
}

function isStreamEvent(value: unknown): value is StreamEvent {
  return typeof value === "object" && value !== null && "type" in value;
}

async function readErrorPayload(response: Response): Promise<unknown> {
  const contentType = response.headers.get("content-type") ?? "";
  if (contentType.includes("application/json")) {
    return response.json();
  }
  return response.text();
}

function delay(milliseconds: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) {
      reject(new TimeoutError("stream retry aborted"));
      return;
    }

    const timeout = setTimeout(resolve, milliseconds);
    signal?.addEventListener(
      "abort",
      () => {
        clearTimeout(timeout);
        reject(new TimeoutError("stream retry aborted"));
      },
      { once: true },
    );
  });
}

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}
