import assert from "node:assert/strict";
import { chmod, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { BinaryNotFoundError, BinaryResolver } from "./resolve-binary.js";

test("BinaryResolver resolves a custom executable path", async () => {
  const directory = await mkdtemp(join(tmpdir(), "sim-sdk-binary-"));
  const binary = join(directory, process.platform === "win32" ? "sim.cmd" : "sim");

  try {
    await writeFile(binary, "#!/bin/sh\nexit 0\n");
    await chmod(binary, 0o755);
    const resolver = new BinaryResolver({ customPath: binary });

    assert.equal(await resolver.resolveBinary(), binary);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("BinaryResolver reports a clear error for missing custom binaries", async () => {
  const resolver = new BinaryResolver({ customPath: "/missing/sim" });

  await assert.rejects(() => resolver.resolveBinary(), (error) => {
    assert.ok(error instanceof BinaryNotFoundError);
    assert.match(error.message, /Sim binary not found/);
    return true;
  });
});
