import type {
  AgentMessageResponse,
  AgentStatus,
  ClientCapabilities,
  ConnectionState,
  CreateSessionOptions,
  GooseClientConfig,
  Message,
  RecipeManifest,
  RecipeOutput,
  RunRecipeRequest,
  Session,
  StreamEvent,
} from "./generated/types.js";
import { createClientCapabilities } from "./client-capabilities.js";
import { HttpTransport, SimError } from "./http-transport.js";
import { HttpStreamClient } from "./stream.js";

export type ConnectionStateCallback = (state: ConnectionState) => void;

export class GooseClient {
  private readonly transport: HttpTransport;
  private readonly streamClient: HttpStreamClient;
  private readonly capabilities: ClientCapabilities;
  private readonly stateCallbacks = new Set<ConnectionStateCallback>();
  private state: ConnectionState = "disconnected";

  constructor(config: GooseClientConfig) {
    if (config.mode === "acp") {
      throw new SimError("ACP mode is not implemented yet; use HTTP mode for now", {
        code: "unsupported_mode",
      });
    }
    if (!config.baseUrl) {
      throw new SimError("baseUrl is required for HTTP SDK connections", {
        code: "invalid_config",
      });
    }

    this.transport = new HttpTransport({
      baseUrl: config.baseUrl,
      apiKey: config.apiKey,
      timeout: config.timeout,
    });
    this.streamClient = new HttpStreamClient({
      baseUrl: config.baseUrl,
      apiKey: config.apiKey,
      timeout: config.timeout,
      maxRetries: config.maxRetries,
    });
    this.capabilities = createClientCapabilities(config.capabilities);
  }

  connectionState(): ConnectionState {
    return this.state;
  }

  onConnectionStateChange(callback: ConnectionStateCallback): () => void {
    this.stateCallbacks.add(callback);
    return () => this.stateCallbacks.delete(callback);
  }

  async connect(): Promise<void> {
    this.setState("connecting");
    try {
      await this.initializeCapabilities();
      await this.transport.getHealth();
      this.setState("connected");
    } catch (error) {
      this.setState("error");
      throw error;
    }
  }

  async disconnect(): Promise<void> {
    this.setState("disconnected");
  }

  createSession(options: CreateSessionOptions = {}): Promise<Session> {
    return this.transport.createSession(options);
  }

  listSessions(): Promise<Session[]> {
    return this.transport.listSessions();
  }

  getSession(id: string): Promise<Session> {
    return this.transport.getSession(id);
  }

  deleteSession(id: string): Promise<void> {
    return this.transport.deleteSession(id);
  }

  sendMessage(sessionId: string, message: Message): Promise<AgentMessageResponse> {
    return this.transport.sendMessage({
      session_id: sessionId,
      message,
      stream: false,
    });
  }

  sendMessageStream(sessionId: string, message: Message): AsyncGenerator<StreamEvent> {
    return this.streamClient.streamMessage({
      session_id: sessionId,
      message,
      stream: true,
    });
  }

  streamSessionEvents(sessionId: string): AsyncGenerator<StreamEvent> {
    return this.streamClient.streamEvents(sessionId);
  }

  getStatus(): Promise<AgentStatus> {
    return this.transport.getStatus();
  }

  listRecipes(): Promise<RecipeManifest[]> {
    return this.transport.listRecipes();
  }

  runRecipe(name: string, variables?: Record<string, string>): Promise<RecipeOutput> {
    const request: RunRecipeRequest = { variables };
    return this.transport.runRecipe(name, request);
  }

  httpTransport(): HttpTransport {
    return this.transport;
  }

  httpStreamClient(): HttpStreamClient {
    return this.streamClient;
  }

  clientCapabilities(): ClientCapabilities {
    return this.capabilities;
  }

  private async initializeCapabilities(): Promise<void> {
    try {
      await this.transport.post<void>("/client/capabilities", this.capabilities);
    } catch {
      // Older servers may not expose the capabilities endpoint yet. Health check
      // remains the compatibility gate for HTTP connections.
    }
  }

  private setState(state: ConnectionState): void {
    if (this.state === state) {
      return;
    }
    this.state = state;
    for (const callback of this.stateCallbacks) {
      callback(state);
    }
  }
}
