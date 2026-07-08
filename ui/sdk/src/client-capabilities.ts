import type { ClientCapabilities, ClientPlatform, Feature } from "./generated/types.js";

const DEFAULT_MAX_MESSAGE_SIZE = 256 * 1024;

export function createClientCapabilities(
  overrides: Partial<ClientCapabilities> = {},
): ClientCapabilities {
  const features = mergeFeatures(defaultFeatures(), overrides.features ?? []);

  return {
    version: overrides.version ?? "0.1.0",
    platform: overrides.platform ?? detectClientPlatform(),
    features,
    streaming: overrides.streaming ?? features.includes("streaming"),
    maxMessageSize: overrides.maxMessageSize ?? DEFAULT_MAX_MESSAGE_SIZE,
  };
}

export function supportsFeature(capabilities: ClientCapabilities, feature: Feature): boolean {
  return capabilities.features.includes(feature);
}

export function mergeFeatures(defaults: Feature[], overrides: Feature[]): Feature[] {
  return Array.from(new Set([...defaults, ...overrides]));
}

export function detectClientPlatform(): ClientPlatform {
  if (typeof process !== "undefined" && process.versions?.node) {
    return "node";
  }
  if (typeof navigator !== "undefined" && /electron/i.test(navigator.userAgent)) {
    return "electron";
  }
  return "web";
}

function defaultFeatures(): Feature[] {
  return ["streaming", "mcp_apps", "recipes"];
}
