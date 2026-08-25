import { CollaborationAuthError } from "./contracts.ts";

export type UnsignedNostrEvent = {
  kind: number;
  created_at: number;
  tags: string[][];
  content: string;
};

export type SignedNostrEvent = UnsignedNostrEvent & {
  id: string;
  pubkey: string;
  sig: string;
};

export type Nip07Provider = {
  getPublicKey(): Promise<string>;
  signEvent(event: UnsignedNostrEvent): Promise<SignedNostrEvent>;
};

type Nip98Environment = {
  crypto: Pick<Crypto, "randomUUID" | "subtle">;
  now: () => number;
};

const HEX_64 = /^[0-9a-f]{64}$/;
const HEX_128 = /^[0-9a-f]{128}$/;

async function sha256Hex(
  value: string,
  crypto: Nip98Environment["crypto"],
): Promise<string> {
  const digest = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(value),
  );
  return Array.from(new Uint8Array(digest), (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("");
}

function equalUnsignedEvent(
  expected: UnsignedNostrEvent,
  actual: SignedNostrEvent,
): boolean {
  return (
    actual.kind === expected.kind &&
    actual.created_at === expected.created_at &&
    actual.content === expected.content &&
    JSON.stringify(actual.tags) === JSON.stringify(expected.tags)
  );
}

function base64Utf8(value: string): string {
  const bytes = new TextEncoder().encode(value);
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}

export async function makeNip98Authorization(
  signer: Nip07Provider | undefined,
  url: string,
  method: "POST",
  body: string,
  environment: Nip98Environment = { crypto, now: Date.now },
): Promise<string> {
  if (!signer) {
    throw new CollaborationAuthError(
      "signer_denied",
      "A NIP-07 browser signer is required to accept this invite.",
    );
  }

  const unsigned: UnsignedNostrEvent = {
    kind: 27_235,
    created_at: Math.floor(environment.now() / 1_000),
    tags: [
      ["u", url],
      ["method", method],
      ["payload", await sha256Hex(body, environment.crypto)],
      ["nonce", environment.crypto.randomUUID()],
    ],
    content: "",
  };

  let expectedPublicKey: string;
  let signed: SignedNostrEvent;
  try {
    expectedPublicKey = await signer.getPublicKey();
    signed = await signer.signEvent(unsigned);
  } catch {
    throw new CollaborationAuthError(
      "signer_denied",
      "The browser signer denied the invite request.",
    );
  }

  if (
    typeof expectedPublicKey !== "string" ||
    signed === null ||
    typeof signed !== "object" ||
    !HEX_64.test(expectedPublicKey) ||
    signed.pubkey !== expectedPublicKey ||
    !HEX_64.test(signed.id) ||
    !HEX_128.test(signed.sig) ||
    !equalUnsignedEvent(unsigned, signed)
  ) {
    throw new CollaborationAuthError(
      "signer_invalid",
      "The browser signer returned an invalid authentication event.",
    );
  }

  const normalized: SignedNostrEvent = {
    ...unsigned,
    id: signed.id,
    pubkey: signed.pubkey,
    sig: signed.sig,
  };
  return `Nostr ${base64Utf8(JSON.stringify(normalized))}`;
}
