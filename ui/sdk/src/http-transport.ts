import type {
  AgentMessageRequest,
  AgentMessageResponse,
  AgentStatus,
  ApiErrorBody,
  ConfigValue,
  CreateSessionOptions,
  HealthStatus,
  RecipeManifest,
  RecipeOutput,
  RunRecipeRequest,
  Session,
} from "./generated/types.js";

export interface HttpTransportOptions {
  baseUrl: string;
  apiKey?: string;
  timeout?: number;
  fetch?: FetchLike;
}

export type FetchLike = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

export class SimError extends Error {
  readonly code?: string;
  readonly status?: number;
  readonly details?: unknown;

  constructor(message: string, options: { code?: string; status?: number; details?: unknown } = {}) {
    super(message);
    this.name = "SimError";
    this.code = options.code;
    this.status = options.status;
    this.details = options.details;
  }
}

export class AuthenticationError extends SimError {
  constructor(message: string, options: { code?: string; status?: number; details?: unknown } = {}) {
    super(message, options);
    this.name = "AuthenticationError";
  }
}

export class TimeoutError extends SimError {
  constructor(message: string) {
    super(message, { code: "timeout" });
    this.name = "TimeoutError";
  }
}

export class HttpTransport {
  readonly baseUrl: string;
  readonly timeout: number;
  private readonly apiKey?: string;
  private readonly fetchImpl: FetchLike;

  constructor(options: HttpTransportOptions) {
    if (!options.baseUrl.trim()) {
      throw new SimError("baseUrl is required", { code: "invalid_config" });
    }

    this.baseUrl = options.baseUrl.replace(/\/+$/, "");
    this.apiKey = options.apiKey;
    this.timeout = options.timeout ?? 30_000;
    this.fetchImpl = options.fetch ?? globalThis.fetch?.bind(globalThis);

    if (!this.fetchImpl) {
      throw new SimError("fetch is not available; provide HttpTransportOptions.fetch", {
        code: "missing_fetch",
      });
    }
  }

  getStatus(): Promise<AgentStatus> {
    return this.get("/agent/status");
  }

  getHealth(): Promise<HealthStatus> {
    return this.get("/health");
  }

  listSessions(): Promise<Session[]> {
    return this.get("/sessions");
  }

  createSession(options: CreateSessionOptions = {}): Promise<Session> {
    return this.post("/sessions", options);
  }

  getSession(id: string): Promise<Session> {
    return this.get(`/sessions/${encodeURIComponent(id)}`);
  }

  deleteSession(id: string): Promise<void> {
    return this.delete(`/sessions/${encodeURIComponent(id)}`);
  }

  sendMessage(request: AgentMessageRequest): Promise<AgentMessageResponse> {
    return this.post("/agent/message", request);
  }

  listRecipes(): Promise<RecipeManifest[]> {
    return this.get("/recipes");
  }

  runRecipe(name: string, request: RunRecipeRequest = {}): Promise<RecipeOutput> {
    return this.post(`/recipes/${encodeURIComponent(name)}/run`, request);
  }

  getConfig(key: string): Promise<ConfigValue> {
    return this.get(`/config/${encodeURIComponent(key)}`);
  }

  setConfig(key: string, value: unknown): Promise<ConfigValue> {
    return this.post(`/config/${encodeURIComponent(key)}`, { value });
  }

  get<T>(path: string): Promise<T> {
    return this.request<T>("GET", path);
  }

  post<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>("POST", path, body);
  }

  delete<T = void>(path: string): Promise<T> {
    return this.request<T>("DELETE", path);
  }

  async request<T>(method: "GET" | "POST" | "DELETE", path: string, body?: unknown): Promise<T> {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), this.timeout);

    try {
      const response = await this.fetchImpl(this.url(path), {
        method,
        headers: this.headers(body),
        body: body === undefined ? undefined : JSON.stringify(body),
        signal: controller.signal,
      });
      return await this.parseResponse<T>(response);
    } catch (error) {
      if (isAbortError(error)) {
        throw new TimeoutError(`Request timed out after ${this.timeout}ms`);
      }
      if (error instanceof SimError) {
        throw error;
      }
      throw new SimError(error instanceof Error ? error.message : "HTTP request failed", {
        code: "request_failed",
      });
    } finally {
      clearTimeout(timeout);
    }
  }

  private url(path: string): string {
    if (/^https?:\/\//.test(path)) {
      return path;
    }
    return `${this.baseUrl}/${path.replace(/^\/+/, "")}`;
  }

  private headers(body: unknown): HeadersInit {
    const headers: Record<string, string> = {
      accept: "application/json",
    };
    if (body !== undefined) {
      headers["content-type"] = "application/json";
    }
    if (this.apiKey) {
      headers.authorization = `Bearer ${this.apiKey}`;
    }
    return headers;
  }

  private async parseResponse<T>(response: Response): Promise<T> {
    if (response.status === 204) {
      return undefined as T;
    }

    const contentType = response.headers.get("content-type") ?? "";
    const payload = contentType.includes("application/json")
      ? await response.json()
      : await response.text();

    if (!response.ok) {
      throw this.errorFromResponse(response, payload);
    }

    return payload as T;
  }

  private errorFromResponse(response: Response, payload: unknown): SimError {
    const body = isApiErrorBody(payload) ? payload : undefined;
    const message = body?.message ?? (response.statusText || `HTTP ${response.status}`);
    const options = {
      code: body?.code,
      status: response.status,
      details: body?.details ?? payload,
    };

    if (response.status === 401 || response.status === 403) {
      return new AuthenticationError(message, options);
    }
    return new SimError(message, options);
  }
}

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}

function isApiErrorBody(payload: unknown): payload is ApiErrorBody {
  return (
    typeof payload === "object" &&
    payload !== null &&
    "message" in payload &&
    typeof (payload as { message: unknown }).message === "string"
  );
}
