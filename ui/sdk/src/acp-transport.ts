import { SimError } from "./http-transport.js";
import { BinaryResolver, type LaunchOptions } from "./resolve-binary.js";

export interface AcpTransportOptions {
  binaryResolver?: BinaryResolver;
  binaryPath?: string;
  args?: string[];
  env?: Record<string, string>;
  cwd?: string;
  requestTimeout?: number;
}

export interface AcpRequestOptions {
  signal?: AbortSignal;
  timeout?: number;
}

export interface AcpResponse<T = unknown> {
  jsonrpc: "2.0";
  id: AcpRequestId;
  result?: T;
  error?: AcpProtocolError;
}

export interface AcpProtocolError {
  code: number;
  message: string;
  data?: unknown;
}

export type AcpRequestId = number | string;
export type AcpNotificationHandler = (method: string, params: unknown) => void;
export type AcpExitHandler = (code: number | null, signal: string | null) => void;

interface AcpMessage {
  jsonrpc?: string;
  id?: AcpRequestId;
  method?: string;
  params?: unknown;
  result?: unknown;
  error?: AcpProtocolError;
}

interface ProcessLike {
  stdin?: WritableLike | null;
  stdout?: ReadableLike | null;
  stderr?: ReadableLike | null;
  killed?: boolean;
  kill(signal?: string): boolean;
  on(event: "exit", callback: (code: number | null, signal: string | null) => void): void;
  on(event: "error", callback: (error: Error) => void): void;
}

interface ReadableLike {
  setEncoding?(encoding: string): void;
  on(event: "data", callback: (chunk: string | Uint8Array) => void): void;
}

interface WritableLike {
  write(chunk: string): boolean;
  end(): void;
}

interface PendingRequest {
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
  timeout: ReturnType<typeof setTimeout>;
  abort?: () => void;
}

export class AcpTransport {
  private readonly binaryResolver: BinaryResolver;
  private readonly args: string[];
  private readonly launchOptions: LaunchOptions;
  private readonly requestTimeout: number;
  private readonly pendingRequests = new Map<AcpRequestId, PendingRequest>();
  private readonly notificationHandlers = new Set<AcpNotificationHandler>();
  private readonly exitHandlers = new Set<AcpExitHandler>();
  private nextRequestId = 1;
  private process?: ProcessLike;
  private stdoutBuffer = "";
  private stderrBuffer = "";
  private closed = false;

  constructor(options: AcpTransportOptions = {}) {
    this.binaryResolver =
      options.binaryResolver ??
      new BinaryResolver(options.binaryPath ? { customPath: options.binaryPath } : {});
    this.args = options.args ?? ["agent", "acp"];
    this.launchOptions = {
      cwd: options.cwd,
      env: options.env,
    };
    this.requestTimeout = options.requestTimeout ?? 30_000;
  }

  async connect(): Promise<void> {
    if (this.process && !this.closed) {
      return;
    }

    this.closed = false;
    const process = (await this.binaryResolver.launchBinary(
      this.args,
      this.launchOptions,
    )) as ProcessLike;
    if (!process.stdin || !process.stdout) {
      throw new SimError("ACP process did not expose stdio pipes", { code: "acp_stdio_missing" });
    }

    this.process = process;
    process.stdout.setEncoding?.("utf8");
    process.stdout.on("data", (chunk) => this.handleStdout(chunk));
    process.stderr?.setEncoding?.("utf8");
    process.stderr?.on("data", (chunk) => this.handleStderr(chunk));
    process.on("error", (error) => this.failAll(error));
    process.on("exit", (code, signal) => {
      this.closed = true;
      this.failAll(new SimError("ACP process exited", {
        code: "acp_process_exited",
        details: {
          exitCode: code,
          signal,
          stderr: this.stderrBuffer.trim() || undefined,
        },
      }));
      for (const callback of this.exitHandlers) {
        callback(code, signal);
      }
    });
  }

  isConnected(): boolean {
    return Boolean(this.process && !this.closed && !this.process.killed);
  }

  onNotification(callback: AcpNotificationHandler): () => void {
    this.notificationHandlers.add(callback);
    return () => this.notificationHandlers.delete(callback);
  }

  onExit(callback: AcpExitHandler): () => void {
    this.exitHandlers.add(callback);
    return () => this.exitHandlers.delete(callback);
  }

  request<T = unknown>(
    method: string,
    params?: unknown,
    options: AcpRequestOptions = {},
  ): Promise<T> {
    if (!method.trim()) {
      throw new SimError("ACP request method is required", { code: "invalid_config" });
    }
    const process = this.requireProcess();
    const id = this.nextRequestId++;
    const payload =
      params === undefined
        ? { jsonrpc: "2.0", id, method }
        : { jsonrpc: "2.0", id, method, params };

    return new Promise<T>((resolve, reject) => {
      if (options.signal?.aborted) {
        reject(new SimError("ACP request aborted", { code: "acp_aborted" }));
        return;
      }

      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new SimError(`ACP request timed out after ${this.timeoutFor(options)}ms`, {
          code: "acp_timeout",
        }));
      }, this.timeoutFor(options));

      const pending: PendingRequest = {
        resolve: (value) => resolve(value as T),
        reject,
        timeout,
      };

      if (options.signal) {
        const abort = () => {
          clearTimeout(timeout);
          this.pendingRequests.delete(id);
          reject(new SimError("ACP request aborted", { code: "acp_aborted" }));
        };
        options.signal.addEventListener("abort", abort, { once: true });
        pending.abort = () => options.signal?.removeEventListener("abort", abort);
      }

      this.pendingRequests.set(id, pending);
      process.stdin?.write(`${JSON.stringify(payload)}\n`);
    });
  }

  notify(method: string, params?: unknown): void {
    if (!method.trim()) {
      throw new SimError("ACP notification method is required", { code: "invalid_config" });
    }
    const process = this.requireProcess();
    const payload =
      params === undefined ? { jsonrpc: "2.0", method } : { jsonrpc: "2.0", method, params };
    process.stdin?.write(`${JSON.stringify(payload)}\n`);
  }

  async close(signal = "SIGTERM"): Promise<void> {
    if (this.closed) {
      return;
    }
    this.closed = true;
    for (const [id, pending] of this.pendingRequests) {
      this.pendingRequests.delete(id);
      clearTimeout(pending.timeout);
      pending.abort?.();
      pending.reject(new SimError("ACP transport closed", { code: "acp_closed" }));
    }
    this.process?.stdin?.end();
    if (this.process && !this.process.killed) {
      this.process.kill(signal);
    }
  }

  private requireProcess(): ProcessLike {
    if (!this.process || this.closed || this.process.killed) {
      throw new SimError("ACP transport is not connected", { code: "acp_not_connected" });
    }
    return this.process;
  }

  private handleStdout(chunk: string | Uint8Array): void {
    this.stdoutBuffer += chunk.toString();
    this.stdoutBuffer = this.drainLines(this.stdoutBuffer, (line) => this.handleLine(line));
  }

  private handleStderr(chunk: string | Uint8Array): void {
    this.stderrBuffer += chunk.toString();
    this.stderrBuffer = this.drainLines(this.stderrBuffer, () => undefined);
  }

  private drainLines(buffer: string, callback: (line: string) => void): string {
    let remaining = buffer;
    for (;;) {
      const newline = remaining.indexOf("\n");
      if (newline === -1) {
        return remaining;
      }
      const line = remaining.slice(0, newline).replace(/\r$/, "");
      remaining = remaining.slice(newline + 1);
      if (line.trim()) {
        callback(line);
      }
    }
  }

  private handleLine(line: string): void {
    let message: AcpMessage;
    try {
      message = JSON.parse(line) as AcpMessage;
    } catch (error) {
      this.failAll(
        new SimError(error instanceof Error ? error.message : "Invalid ACP JSON message", {
          code: "acp_parse_error",
          details: line,
        }),
      );
      return;
    }

    if (message.id !== undefined) {
      this.handleResponse(message);
      return;
    }
    if (message.method) {
      for (const callback of this.notificationHandlers) {
        callback(message.method, message.params);
      }
    }
  }

  private handleResponse(message: AcpMessage): void {
    if (message.id === undefined) {
      return;
    }
    const pending = this.pendingRequests.get(message.id);
    if (!pending) {
      return;
    }

    this.pendingRequests.delete(message.id);
    clearTimeout(pending.timeout);
    pending.abort?.();
    if (message.error) {
      pending.reject(
        new SimError(message.error.message, {
          code: `acp_${message.error.code}`,
          details: message.error.data,
        }),
      );
      return;
    }
    pending.resolve(message.result);
  }

  private failAll(error: Error): void {
    for (const [id, pending] of this.pendingRequests) {
      this.pendingRequests.delete(id);
      clearTimeout(pending.timeout);
      pending.abort?.();
      pending.reject(error);
    }
  }

  private timeoutFor(options: AcpRequestOptions): number {
    return options.timeout ?? this.requestTimeout;
  }
}
