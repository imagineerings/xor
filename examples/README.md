# Sim Agent Examples

This directory contains small, dependency-free examples for agent extension workflows.

## MCP wiki integration

Run a local JSON-RPC MCP-style server over stdio:

```sh
node examples/mcp-wiki-integration/wiki-mcp-server.js
```

Send newline-delimited JSON-RPC requests such as:

```json
{"jsonrpc":"2.0","id":1,"method":"tools/list"}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"search_wiki","arguments":{"query":"agent"}}}
```

## Plugin usage

Inspect and run the example plugin manifest:

```sh
node examples/plugin-usage/run-plugin.js examples/plugin-usage/plugin.json
```

## Frontend tools

Open `examples/frontend-tools/index.html` in a browser. The page calls the local demo tool runtime in `tool-client.js` without any build step.
