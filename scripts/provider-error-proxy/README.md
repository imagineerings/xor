# Provider Error Proxy

`provider-error-proxy` is a local debugging proxy for LLM provider requests. It
logs request and response metadata, captures small body previews, redacts
sensitive headers by default, and forwards traffic to the configured provider.

## Usage

```sh
node scripts/provider-error-proxy/proxy.js \
  --target https://api.openai.com \
  --listen 127.0.0.1:8787 \
  --log /tmp/provider-error-proxy.jsonl
```

Point an SDK at `http://127.0.0.1:8787` while keeping the same paths it would
send to the provider. For example, a request to:

```text
http://127.0.0.1:8787/v1/chat/completions
```

is forwarded to:

```text
https://api.openai.com/v1/chat/completions
```

Each request writes two JSONL events:

- `request`: method, URL, redacted headers, and request body preview.
- `response`: status, duration, redacted headers, and response body preview.

Proxy failures write an `error` event and return `502`.

## Options

```text
--target URL              Provider base URL. Defaults to PROVIDER_PROXY_TARGET.
--listen HOST:PORT        Listen address. Defaults to 127.0.0.1:8787.
--log PATH                JSONL log path. Defaults to provider-error-proxy.jsonl.
--max-body-bytes BYTES    Max body bytes retained in logs. Defaults to 65536.
--show-sensitive-headers  Do not redact authorization/cookie/api-key headers.
```
