#!/usr/bin/env node

const fs = require("node:fs");
const http = require("node:http");
const https = require("node:https");
const { URL } = require("node:url");

const DEFAULT_LISTEN = "127.0.0.1:8787";
const DEFAULT_LOG_PATH = "provider-error-proxy.jsonl";
const DEFAULT_MAX_BODY_BYTES = 64 * 1024;
const SENSITIVE_HEADER_PATTERN = /authorization|cookie|api[-_]?key|token|secret/i;

function usage() {
  console.log(`usage: node scripts/provider-error-proxy/proxy.js --target <url> [options]

options:
  --target URL              Provider base URL. Defaults to PROVIDER_PROXY_TARGET.
  --listen HOST:PORT        Listen address. Defaults to ${DEFAULT_LISTEN}.
  --log PATH                JSONL log path. Defaults to ${DEFAULT_LOG_PATH}.
  --max-body-bytes BYTES    Max body bytes retained in logs. Defaults to ${DEFAULT_MAX_BODY_BYTES}.
  --show-sensitive-headers  Do not redact authorization/cookie/api-key headers.
  --help                    Show this help message.`);
}

function parseArguments(argv) {
  const options = {
    target: process.env.PROVIDER_PROXY_TARGET,
    listen: DEFAULT_LISTEN,
    logPath: DEFAULT_LOG_PATH,
    maxBodyBytes: DEFAULT_MAX_BODY_BYTES,
    redactSensitiveHeaders: true,
  };

  for (let index = 2; index < argv.length; index += 1) {
    const argument = argv[index];
    switch (argument) {
      case "--target":
        options.target = argv[++index];
        break;
      case "--listen":
        options.listen = argv[++index];
        break;
      case "--log":
        options.logPath = argv[++index];
        break;
      case "--max-body-bytes":
        options.maxBodyBytes = Number(argv[++index]);
        break;
      case "--show-sensitive-headers":
        options.redactSensitiveHeaders = false;
        break;
      case "--help":
      case "-h":
        options.help = true;
        break;
      default:
        throw new Error(`unknown option: ${argument}`);
    }
  }

  if (options.help) {
    return options;
  }
  if (!options.target) {
    throw new Error("--target or PROVIDER_PROXY_TARGET is required");
  }
  if (!Number.isInteger(options.maxBodyBytes) || options.maxBodyBytes < 0) {
    throw new Error("--max-body-bytes must be a non-negative integer");
  }
  options.target = normalizeTargetUrl(options.target);
  options.listen = parseListenAddress(options.listen);
  return options;
}

function normalizeTargetUrl(rawTarget) {
  const target = new URL(rawTarget);
  if (target.protocol !== "http:" && target.protocol !== "https:") {
    throw new Error("--target must use http or https");
  }
  target.pathname = target.pathname.replace(/\/+$/, "");
  target.search = "";
  target.hash = "";
  return target;
}

function parseListenAddress(rawListen) {
  const separatorIndex = rawListen.lastIndexOf(":");
  if (separatorIndex <= 0) {
    throw new Error("--listen must use HOST:PORT");
  }
  const host = rawListen.slice(0, separatorIndex);
  const port = Number(rawListen.slice(separatorIndex + 1));
  if (!Number.isInteger(port) || port <= 0 || port > 65535) {
    throw new Error("--listen port must be between 1 and 65535");
  }
  return { host, port };
}

function redactHeaders(headers, redactSensitiveHeaders) {
  const redacted = {};
  for (const [name, value] of Object.entries(headers)) {
    if (redactSensitiveHeaders && SENSITIVE_HEADER_PATTERN.test(name)) {
      redacted[name] = "<redacted>";
    } else {
      redacted[name] = value;
    }
  }
  return redacted;
}

function bodyPreview(buffer, maxBodyBytes) {
  const truncated = buffer.length > maxBodyBytes;
  const visible = truncated ? buffer.subarray(0, maxBodyBytes) : buffer;
  return {
    bytes: buffer.length,
    truncated,
    text: visible.toString("utf8"),
  };
}

function readRequestBody(request, maxBodyBytes) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let totalBytes = 0;
    request.on("data", (chunk) => {
      totalBytes += chunk.length;
      chunks.push(chunk);
      if (totalBytes > maxBodyBytes * 16 && maxBodyBytes > 0) {
        reject(new Error(`request body exceeded capture safety limit: ${totalBytes} bytes`));
        request.destroy();
      }
    });
    request.on("end", () => resolve(Buffer.concat(chunks)));
    request.on("error", reject);
  });
}

function writeEvent(logStream, event) {
  logStream.write(`${JSON.stringify({ timestamp: new Date().toISOString(), ...event })}\n`);
}

function buildTargetUrl(target, requestUrl) {
  const request = new URL(requestUrl, "http://proxy.local");
  const forwarded = new URL(target.href);
  const targetPath = target.pathname === "/" ? "" : target.pathname;
  forwarded.pathname = `${targetPath}${request.pathname}`;
  forwarded.search = request.search;
  return forwarded;
}

function forwardRequest(targetUrl, incomingRequest, requestBody) {
  const transport = targetUrl.protocol === "https:" ? https : http;
  const headers = { ...incomingRequest.headers, host: targetUrl.host };
  return new Promise((resolve, reject) => {
    const upstreamRequest = transport.request(
      targetUrl,
      {
        method: incomingRequest.method,
        headers,
      },
      (upstreamResponse) => {
        const chunks = [];
        upstreamResponse.on("data", (chunk) => chunks.push(chunk));
        upstreamResponse.on("end", () => {
          resolve({
            statusCode: upstreamResponse.statusCode || 502,
            statusMessage: upstreamResponse.statusMessage || "",
            headers: upstreamResponse.headers,
            body: Buffer.concat(chunks),
          });
        });
      },
    );
    upstreamRequest.on("error", reject);
    upstreamRequest.end(requestBody);
  });
}

function createServer(options) {
  const logStream = fs.createWriteStream(options.logPath, { flags: "a" });

  const server = http.createServer(async (request, response) => {
    const startedAt = process.hrtime.bigint();
    const requestBody = await readRequestBody(request, options.maxBodyBytes).catch((error) => {
      writeEvent(logStream, {
        type: "error",
        phase: "request",
        method: request.method,
        url: request.url,
        error: error.message,
      });
      response.writeHead(413, { "content-type": "application/json" });
      response.end(JSON.stringify({ error: error.message }));
      return null;
    });
    if (requestBody === null) {
      return;
    }

    const targetUrl = buildTargetUrl(options.target, request.url || "/");
    writeEvent(logStream, {
      type: "request",
      method: request.method,
      url: request.url,
      target: targetUrl.href,
      headers: redactHeaders(request.headers, options.redactSensitiveHeaders),
      body: bodyPreview(requestBody, options.maxBodyBytes),
    });

    try {
      const upstreamResponse = await forwardRequest(targetUrl, request, requestBody);
      response.writeHead(upstreamResponse.statusCode, upstreamResponse.statusMessage, upstreamResponse.headers);
      response.end(upstreamResponse.body);

      const durationMs = Number(process.hrtime.bigint() - startedAt) / 1_000_000;
      writeEvent(logStream, {
        type: "response",
        method: request.method,
        url: request.url,
        target: targetUrl.href,
        status: upstreamResponse.statusCode,
        duration_ms: Number(durationMs.toFixed(3)),
        headers: redactHeaders(upstreamResponse.headers, options.redactSensitiveHeaders),
        body: bodyPreview(upstreamResponse.body, options.maxBodyBytes),
      });
    } catch (error) {
      response.writeHead(502, { "content-type": "application/json" });
      response.end(JSON.stringify({ error: error.message }));
      writeEvent(logStream, {
        type: "error",
        phase: "forward",
        method: request.method,
        url: request.url,
        target: targetUrl.href,
        error: error.message,
      });
    }
  });

  server.on("close", () => logStream.end());
  return server;
}

function main() {
  const options = parseArguments(process.argv);
  if (options.help) {
    usage();
    return;
  }

  const server = createServer(options);
  server.listen(options.listen.port, options.listen.host, () => {
    console.log(
      `provider-error-proxy listening on http://${options.listen.host}:${options.listen.port} -> ${options.target.href}`,
    );
    console.log(`logging JSONL events to ${options.logPath}`);
  });
}

if (require.main === module) {
  try {
    main();
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }
}

module.exports = {
  bodyPreview,
  buildTargetUrl,
  createServer,
  parseArguments,
  redactHeaders,
};
