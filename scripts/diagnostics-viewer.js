#!/usr/bin/env node

const fs = require("node:fs");

function usage() {
  console.log("usage: node scripts/diagnostics-viewer.js <diagnostics.json|diagnostics.jsonl|-> [--limit N]");
}

function parseArguments(argv) {
  const args = { source: argv[2], limit: 20 };
  for (let index = 3; index < argv.length; index += 1) {
    if (argv[index] === "--limit") {
      args.limit = Number(argv[index + 1]);
      index += 1;
    }
  }
  return args;
}

function readInput(source) {
  if (!source || source === "-") {
    return fs.readFileSync(0, "utf8");
  }
  return fs.readFileSync(source, "utf8");
}

function parseDiagnostics(raw) {
  const trimmed = raw.trim();
  if (!trimmed) {
    return [];
  }
  try {
    const parsed = JSON.parse(trimmed);
    if (Array.isArray(parsed)) {
      return parsed;
    }
    if (Array.isArray(parsed.diagnostics)) {
      return parsed.diagnostics;
    }
    return [parsed];
  } catch {
    return trimmed.split(/\r?\n/).filter(Boolean).map((line) => {
      try {
        return JSON.parse(line);
      } catch {
        return { level: "info", message: line };
      }
    });
  }
}

function severityOf(diagnostic) {
  return String(
    diagnostic.severity ||
      diagnostic.level ||
      diagnostic.kind ||
      diagnostic.type ||
      "info",
  ).toLowerCase();
}

function messageOf(diagnostic) {
  return String(
    diagnostic.message ||
      diagnostic.text ||
      diagnostic.summary ||
      diagnostic.reason ||
      JSON.stringify(diagnostic),
  );
}

function locationOf(diagnostic) {
  const file = diagnostic.file || diagnostic.path || diagnostic.uri;
  const line = diagnostic.line || diagnostic.row || diagnostic.startLine;
  if (!file) {
    return "";
  }
  return line ? `${file}:${line}` : String(file);
}

function main() {
  if (process.argv.includes("--help")) {
    usage();
    return;
  }
  const args = parseArguments(process.argv);
  const diagnostics = parseDiagnostics(readInput(args.source));
  const counts = new Map();
  for (const diagnostic of diagnostics) {
    const severity = severityOf(diagnostic);
    counts.set(severity, (counts.get(severity) || 0) + 1);
  }
  console.log(`Diagnostics: ${diagnostics.length}`);
  for (const [severity, count] of [...counts.entries()].sort()) {
    console.log(`  ${severity}: ${count}`);
  }
  for (const diagnostic of diagnostics.slice(0, args.limit)) {
    const location = locationOf(diagnostic);
    const prefix = location ? `${location} ` : "";
    console.log(`- [${severityOf(diagnostic)}] ${prefix}${messageOf(diagnostic)}`);
  }
  if (diagnostics.length > args.limit) {
    console.log(`... ${diagnostics.length - args.limit} more`);
  }
}

main();
