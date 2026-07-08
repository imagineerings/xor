// Generated type scaffold for the Sim REST API.
// This file is intentionally checked in so SDK implementation work can depend
// on stable names while the OpenAPI schema matures.

export type ConnectionMode = "http" | "acp" | "auto";
export type ConnectionState = "disconnected" | "connecting" | "connected" | "error";
export type MessageRole = "user" | "assistant" | "system" | "tool";
export type SessionStatus = "active" | "archived";

export interface GooseClientConfig {
  mode: ConnectionMode;
  baseUrl?: string;
  apiKey?: string;
  binaryPath?: string;
  timeout?: number;
  maxRetries?: number;
  capabilities?: Partial<ClientCapabilities>;
}

export interface ContentBlock {
  type: string;
  text?: string;
  data?: unknown;
}

export interface Message {
  role: MessageRole;
  content: string | ContentBlock[];
  id?: string;
  timestamp?: string;
}

export interface Session {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  message_count: number;
  status: SessionStatus;
}

export interface CreateSessionOptions {
  title?: string;
  metadata?: Record<string, unknown>;
}

export interface AgentMessageRequest {
  session_id: string;
  message: Message;
  stream?: boolean;
}

export interface AgentMessageResponse {
  session: Session;
  message: Message;
  output?: string;
}

export interface AgentStatus {
  ok: boolean;
  version?: string;
  active_sessions?: number;
}

export interface ToolCallInfo {
  id: string;
  name: string;
  arguments?: unknown;
}

export type StreamEvent =
  | { type: "message"; message: Message }
  | { type: "tool_call"; toolCall: ToolCallInfo }
  | { type: "tool_result"; toolCallId: string; result: unknown }
  | { type: "error"; error: string }
  | { type: "done" };

export interface RecipeManifest {
  name: string;
  title?: string;
  description?: string;
  variables?: Record<string, string>;
}

export interface RecipeOutput {
  name: string;
  output: string;
  metadata?: Record<string, unknown>;
}

export interface RunRecipeRequest {
  variables?: Record<string, string>;
}

export interface ConfigValue {
  key: string;
  value: unknown;
}

export interface HealthStatus {
  ok: boolean;
}

export interface MCPTool {
  name: string;
  description?: string;
  input_schema?: unknown;
}

export interface RegisterMCPToolRequest {
  tool: MCPTool;
}

export interface CallMCPToolRequest {
  name: string;
  arguments?: unknown;
}

export interface CallMCPToolResponse {
  result: unknown;
}

export interface BinaryResolverOptions {
  customPath?: string;
  version?: string;
  allowDownload?: boolean;
}

export type ClientPlatform = "web" | "node" | "electron";
export type Feature = "streaming" | "mcp_apps" | "recipes" | "scheduling" | "dictation";

export interface ClientCapabilities {
  version: string;
  platform: ClientPlatform;
  features: Feature[];
  streaming: boolean;
  maxMessageSize: number;
}

export interface ApiErrorBody {
  code?: string;
  message: string;
  details?: unknown;
}
