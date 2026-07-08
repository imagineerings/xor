import type {
  CallMCPToolRequest,
  CallMCPToolResponse,
  MCPTool,
  RegisterMCPToolRequest,
} from "./generated/types.js";
import type { GooseClient } from "./goose-client.js";
import { HttpTransport, SimError } from "./http-transport.js";

export type MCPAppsClientTarget = GooseClient | HttpTransport;

export class MCPAppsClient {
  private readonly transport: HttpTransport;
  private running = false;

  constructor(target: MCPAppsClientTarget) {
    this.transport = target instanceof HttpTransport ? target : target.httpTransport();
  }

  async start(): Promise<void> {
    await this.transport.post<void>("/mcp/apps/start");
    this.running = true;
  }

  async stop(): Promise<void> {
    await this.transport.post<void>("/mcp/apps/stop");
    this.running = false;
  }

  isRunning(): boolean {
    return this.running;
  }

  listTools(): Promise<MCPTool[]> {
    return this.transport.get<MCPTool[]>("/mcp/apps/tools");
  }

  async registerTool(tool: MCPTool): Promise<void> {
    validateTool(tool);
    const request: RegisterMCPToolRequest = { tool };
    await this.transport.post<void>("/mcp/apps/tools", request);
  }

  async unregisterTool(name: string): Promise<void> {
    validateToolName(name);
    await this.transport.delete<void>(`/mcp/apps/tools/${encodeURIComponent(name)}`);
  }

  async callTool(name: string, args?: unknown): Promise<unknown> {
    validateToolName(name);
    const request: CallMCPToolRequest = {
      name,
      arguments: args,
    };
    const response = await this.transport.post<CallMCPToolResponse>("/mcp/apps/tools/call", request);
    return response.result;
  }
}

function validateTool(tool: MCPTool): void {
  validateToolName(tool.name);
  if (tool.description !== undefined && !tool.description.trim()) {
    throw new SimError("MCP tool description cannot be empty when provided", {
      code: "invalid_tool",
    });
  }
}

function validateToolName(name: string): void {
  if (!name.trim()) {
    throw new SimError("MCP tool name is required", { code: "invalid_tool" });
  }
}
