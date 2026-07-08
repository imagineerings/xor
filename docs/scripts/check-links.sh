#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

node <<'NODE'
const fs = require("fs");
const path = require("path");

const roots = ["docs/src", "docs/tutorials", "docs/blog"];
const failures = [];

function markdownFiles(directory) {
  if (!fs.existsSync(directory)) return [];
  const entries = fs.readdirSync(directory, { withFileTypes: true });
  return entries.flatMap((entry) => {
    const fullPath = path.join(directory, entry.name);
    if (entry.isDirectory()) return markdownFiles(fullPath);
    return entry.isFile() && /\.mdx?$/.test(entry.name) ? [fullPath] : [];
  });
}

for (const file of roots.flatMap(markdownFiles)) {
  const text = fs.readFileSync(file, "utf8");
  for (const match of text.matchAll(/\[[^\]]+\]\(([^)#]+)(?:#[^)]+)?\)/g)) {
    const href = match[1];
    if (/^(https?:|mailto:|sim:)/.test(href)) continue;
    if (!href.startsWith(".")) continue;

    const target = path.normalize(path.join(path.dirname(file), href));
    const candidates = [target, `${target}.md`, `${target}.mdx`, path.join(target, "index.md")];
    if (!candidates.some((candidate) => fs.existsSync(candidate))) {
      failures.push(`${file}: missing ${href}`);
    }
  }
}

if (failures.length) {
  console.error(failures.join("\n"));
  process.exit(1);
}

console.log("docs links ok");
NODE
