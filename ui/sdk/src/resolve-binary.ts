import type { BinaryResolverOptions } from "./generated/types.js";
import { SimError } from "./http-transport.js";

export interface LaunchOptions {
  args?: string[];
  env?: Record<string, string>;
  cwd?: string;
}

export interface BinaryVersion {
  path: string;
  version: string;
}

type ChildProcessLike = unknown;

export class BinaryNotFoundError extends SimError {
  constructor(message: string) {
    super(message, { code: "binary_not_found" });
    this.name = "BinaryNotFoundError";
  }
}

export class BinaryResolver {
  private customPath?: string;
  private readonly version?: string;
  private readonly allowDownload: boolean;

  constructor(options: BinaryResolverOptions = {}) {
    this.customPath = options.customPath;
    this.version = options.version;
    this.allowDownload = options.allowDownload ?? false;
  }

  setCustomPath(path: string): void {
    if (!path.trim()) {
      throw new SimError("custom binary path cannot be empty", { code: "invalid_config" });
    }
    this.customPath = path;
  }

  async resolveBinary(): Promise<string> {
    const candidates = await this.candidates();
    for (const candidate of candidates) {
      if (await isExecutable(candidate)) {
        return candidate;
      }
    }

    if (this.allowDownload) {
      throw new BinaryNotFoundError(
        "Sim binary download is not implemented yet; provide a custom binary path",
      );
    }
    throw new BinaryNotFoundError(
      `Sim binary not found. Checked: ${candidates.join(", ") || "no candidates"}`,
    );
  }

  async launchBinary(args: string[] = [], options: LaunchOptions = {}): Promise<ChildProcessLike> {
    const binary = await this.resolveBinary();
    const childProcess = await import("node:child_process");
    return childProcess.spawn(binary, [...args, ...(options.args ?? [])], {
      cwd: options.cwd,
      env: {
        ...process.env,
        ...options.env,
      },
      stdio: ["pipe", "pipe", "pipe"],
    });
  }

  async findInstalledVersion(): Promise<BinaryVersion | null> {
    const binary = await this.resolveBinary().catch(() => null);
    if (!binary) {
      return null;
    }

    const childProcess = await import("node:child_process");
    const version = await new Promise<string>((resolve, reject) => {
      childProcess.execFile(binary, ["--version"], (error, stdout, stderr) => {
        if (error) {
          reject(error);
          return;
        }
        resolve((stdout || stderr).trim());
      });
    });

    return { path: binary, version };
  }

  private async candidates(): Promise<string[]> {
    if (this.customPath) {
      return [this.customPath];
    }

    const path = await import("node:path");
    const os = await import("node:os");
    const platform = process.platform;
    const executable = platform === "win32" ? "sim.exe" : "sim";
    const versionSuffix = this.version ? `-${this.version}` : "";
    const home = os.homedir();

    return [
      process.env.SIM_BINARY,
      path.join(process.cwd(), "target", "release", executable),
      path.join(process.cwd(), "target", "debug", executable),
      path.join(home, ".sim", "bin", `sim${versionSuffix}`, executable),
      path.join(home, ".local", "bin", executable),
      path.join("/usr/local/bin", executable),
      executable,
    ].filter((candidate): candidate is string => Boolean(candidate));
  }
}

async function isExecutable(candidate: string): Promise<boolean> {
  const fs = await import("node:fs/promises");
  const constants = await import("node:fs");
  try {
    await fs.access(candidate, constants.constants.X_OK);
    return true;
  } catch {
    return false;
  }
}
