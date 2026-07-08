#!/usr/bin/env node

const assert = require("node:assert");
const fs = require("node:fs");
const { spawn, spawnSync } = require("node:child_process");
const { bodyPreview, buildTargetUrl, redactHeaders } = require("./provider-error-proxy/proxy.js");

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd || process.cwd(),
    encoding: "utf8",
    input: options.input,
  });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`,
    );
  }
  return result.stdout;
}

async function runWikiSmokeTest() {
  const child = spawn("node", ["examples/mcp-wiki-integration/wiki-mcp-server.js"], {
    stdio: ["pipe", "pipe", "pipe"],
  });
  let stdout = "";
  let stderr = "";
  child.stdout.on("data", (chunk) => {
    stdout += chunk;
  });
  child.stderr.on("data", (chunk) => {
    stderr += chunk;
  });
  child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id: 1, method: "tools/list" })}\n`);
  child.stdin.write(
    `${JSON.stringify({
      jsonrpc: "2.0",
      id: 2,
      method: "tools/call",
      params: { name: "search_wiki", arguments: { query: "agent" } },
    })}\n`,
  );
  await new Promise((resolve) => setTimeout(resolve, 100));
  child.kill();
  if (stderr) {
    throw new Error(stderr);
  }
  const responses = stdout.trim().split(/\n/).filter(Boolean).map(JSON.parse);
  assert.equal(responses.length, 2);
  assert(responses[0].result.tools.some((tool) => tool.name === "search_wiki"));
  assert(responses[1].result.content[0].text.includes("Agent Overview"));
}

function verifyPluginExample() {
  const output = run("node", [
    "examples/plugin-usage/run-plugin.js",
    "examples/plugin-usage/plugin.json",
  ]);
  const parsed = JSON.parse(output);
  assert.equal(parsed.plugin, "sim.example.agent-helper");
  assert.deepEqual(parsed.commands, ["example.sayHello"]);
  assert.deepEqual(parsed.mcpServers, ["example-wiki"]);
}

function verifyFrontendExample() {
  const html = fs.readFileSync("examples/frontend-tools/index.html", "utf8");
  const script = fs.readFileSync("examples/frontend-tools/tool-client.js", "utf8");
  assert(html.includes("./tool-client.js"));
  assert(script.includes("summarize_text"));
}

function verifyOpenApiValidator() {
  const document = {
    openapi: "3.1.0",
    info: { title: "Sim", version: "0.1.0" },
    paths: {
      "/health": {
        get: {
          responses: {
            200: {
              description: "ok",
              content: {
                "application/json": {
                  schema: { type: "object", properties: { ok: { type: "boolean" } } },
                },
              },
            },
          },
        },
      },
    },
  };
  const output = run("node", ["scripts/validate-openapi-schema.js", "-"], {
    input: JSON.stringify(document),
  });
  assert(output.includes("OpenAPI schema ok"));
}

function verifyDiagnosticsViewer() {
  const input = [
    JSON.stringify({ severity: "error", file: "src/main.rs", line: 12, message: "example failure" }),
    JSON.stringify({ level: "warning", message: "example warning" }),
  ].join("\n");
  const output = run("node", ["scripts/diagnostics-viewer.js", "-", "--limit", "5"], { input });
  assert(output.includes("Diagnostics: 2"));
  assert(output.includes("[error] src/main.rs:12 example failure"));
}

function verifyProviderProxyHelpers() {
  assert.equal(
    buildTargetUrl(new URL("https://api.example.test/base"), "/v1/chat?x=1").href,
    "https://api.example.test/base/v1/chat?x=1",
  );
  assert.deepEqual(redactHeaders({ authorization: "secret", "x-name": "ok" }, true), {
    authorization: "<redacted>",
    "x-name": "ok",
  });
  assert.deepEqual(bodyPreview(Buffer.from("abcdef"), 3), {
    bytes: 6,
    truncated: true,
    text: "abc",
  });
}

async function main() {
  run("bash", [
    "-n",
    "scripts/database-helper.sh",
    "scripts/test-mcp-servers.sh",
    "scripts/test-sub-agent-and-recipe.sh",
    "scripts/test-compaction.sh",
    "scripts/prerelease-check.sh",
    "scripts/test-misc-services.sh",
    "scripts/bench-agent.sh",
  ]);
  run("node", ["--check", "scripts/validate-openapi-schema.js"]);
  run("node", ["--check", "scripts/diagnostics-viewer.js"]);
  run("node", ["--check", "scripts/provider-error-proxy/proxy.js"]);
  verifyPluginExample();
  verifyFrontendExample();
  verifyOpenApiValidator();
  verifyDiagnosticsViewer();
  verifyProviderProxyHelpers();
  await runWikiSmokeTest();
  console.log("misc services verification ok");
}

main().catch((error) => {
  console.error(error.stack || error.message);
  process.exit(1);
});
