#!/usr/bin/env node

const readline = require("node:readline");

const pages = [
  {
    id: "agent-overview",
    title: "Agent Overview",
    body: "The Sim agent can read files, edit code, run commands, and use MCP tools.",
  },
  {
    id: "mcp-tools",
    title: "MCP Tools",
    body: "MCP servers expose tools over JSON-RPC. Sim can launch local MCP servers over stdio.",
  },
  {
    id: "recipes",
    title: "Recipes",
    body: "Recipes package repeatable agent workflows with prompts, inputs, and validation steps.",
  },
];

const tools = [
  {
    name: "search_wiki",
    description: "Search the local example wiki.",
    input_schema: {
      type: "object",
      properties: { query: { type: "string" } },
      required: ["query"],
    },
  },
  {
    name: "get_page",
    description: "Read one wiki page by id.",
    input_schema: {
      type: "object",
      properties: { id: { type: "string" } },
      required: ["id"],
    },
  },
];

function handleRequest(request) {
  if (request.method === "initialize") {
    return {
      protocolVersion: "2025-11-25",
      capabilities: { tools: { listChanged: false } },
      serverInfo: { name: "sim-example-wiki", version: "0.1.0" },
    };
  }
  if (request.method === "tools/list") {
    return { tools };
  }
  if (request.method === "tools/call") {
    const params = request.params || {};
    const name = params.name;
    const args = params.arguments || {};
    if (name === "search_wiki") {
      const query = String(args.query || "").toLowerCase();
      const matches = pages.filter(
        (page) =>
          page.title.toLowerCase().includes(query) ||
          page.body.toLowerCase().includes(query),
      );
      return toolResult({ matches });
    }
    if (name === "get_page") {
      const page = pages.find((candidate) => candidate.id === args.id);
      if (!page) throw new Error(`unknown wiki page: ${args.id}`);
      return toolResult({ page });
    }
    throw new Error(`unknown tool: ${name}`);
  }
  if (request.method === "ping") {
    return {};
  }
  throw new Error(`unknown method: ${request.method}`);
}

function toolResult(value) {
  return {
    content: [{ type: "text", text: JSON.stringify(value) }],
    isError: false,
  };
}

function respond(id, result) {
  process.stdout.write(`${JSON.stringify({ jsonrpc: "2.0", id, result })}\n`);
}

function respondError(id, error) {
  process.stdout.write(
    `${JSON.stringify({
      jsonrpc: "2.0",
      id,
      error: { code: -32603, message: error.message },
    })}\n`,
  );
}

const input = readline.createInterface({ input: process.stdin });
input.on("line", (line) => {
  if (!line.trim()) return;
  let request;
  try {
    request = JSON.parse(line);
  } catch (error) {
    respondError(null, error);
    return;
  }
  try {
    respond(request.id ?? null, handleRequest(request));
  } catch (error) {
    respondError(request.id ?? null, error);
  }
});
