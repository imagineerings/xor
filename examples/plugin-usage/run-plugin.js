#!/usr/bin/env node

const fs = require("node:fs");

const manifestPath = process.argv[2];
if (!manifestPath) {
  console.error("usage: node examples/plugin-usage/run-plugin.js <plugin.json>");
  process.exit(2);
}

const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
const commands = (manifest.commands || []).map((command) => command.id);
const mcpServers = (manifest.mcpServers || []).map((server) => server.id);

console.log(
  JSON.stringify(
    {
      plugin: manifest.id,
      name: manifest.name,
      commands,
      mcpServers,
    },
    null,
    2,
  ),
);
