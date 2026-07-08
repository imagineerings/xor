# MCP Wiki Integration

`wiki-mcp-server.js` is a tiny stdio JSON-RPC server with two tools:

- `search_wiki` searches embedded pages.
- `get_page` returns one page by id.

Run it with Node.js and send newline-delimited JSON-RPC requests on stdin.
