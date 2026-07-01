/**
 * Baymax OIDC Proxy — Cloudflare Worker
 *
 * Routes OIDC (OpenID Connect) authentication flows through a proxy so that
 * the client (baymax) never needs to hold the provider's client secret.
 *
 * ## Routes
 *
 * | Method | Path                                | Description                        |
 * |--------|-------------------------------------|------------------------------------|
 * | GET    | /authorize                          | Initiate OIDC flow (redirect)      |
 * | GET    | /callback                           | Handle OIDC authorization callback |
 * | POST   | /token                              | Exchange code for tokens           |
 * | GET    | /.well-known/openid-configuration   | OIDC discovery document            |
 *
 * ## Environment variables (secrets)
 *
 * - `OIDC_CLIENT_ID`       — OAuth client identifier (via `wrangler secret put`)
 * - `OIDC_CLIENT_SECRET`   — OAuth client secret   (via `wrangler secret put`)
 *
 * ## Environment variables (plain)
 *
 * - `OIDC_PROVIDER_AUTHORIZE_URL` — Upstream authorise endpoint
 * - `OIDC_PROVIDER_TOKEN_URL`     — Upstream token endpoint
 * - `OIDC_ISSUER`                 — issuer reported in well-known config
 */

// ── Types ────────────────────────────────────────────────────────────────

export interface Env {
	/** OAuth client identifier (secret). */
	OIDC_CLIENT_ID: string;
	/** OAuth client secret (secret). */
	OIDC_CLIENT_SECRET: string;
	/** Upstream OIDC provider authorisation endpoint URL. */
	OIDC_PROVIDER_AUTHORIZE_URL: string;
	/** Upstream OIDC provider token endpoint URL. */
	OIDC_PROVIDER_TOKEN_URL: string;
	/** Issuer URL reported in the well-known configuration. */
	OIDC_ISSUER: string;
}

// ── Route helpers ────────────────────────────────────────────────────────

/** Known route patterns. */
type Route =
	| { kind: "authorize" }
	| { kind: "callback" }
	| { kind: "token" }
	| { kind: "wellKnown" }
	| { kind: "notFound" };

function matchRoute(request: Request): Route {
	const url = new URL(request.url);
	switch (url.pathname) {
		case "/authorize":
			return { kind: "authorize" };
		case "/callback":
			return { kind: "callback" };
		case "/token":
			return { kind: "token" };
		case "/.well-known/openid-configuration":
			return { kind: "wellKnown" };
		default:
			return { kind: "notFound" };
	}
}

// ── Error responses ──────────────────────────────────────────────────────

/** Return a JSON error response. */
function errorResponse(status: number, error: string, description?: string): Response {
	const body: Record<string, string> = { error };
	if (description) body.error_description = description;
	return new Response(JSON.stringify(body, null, 2), {
		status,
		headers: { "content-type": "application/json; charset=utf-8" },
	});
}

// ── Route handlers ───────────────────────────────────────────────────────

/**
 * GET /authorize
 *
 * Redirect the user-agent to the upstream OIDC provider's authorisation
 * endpoint, passing through standard OAuth2 parameters.
 */
function handleAuthorize(request: Request, env: Env): Response {
	const url = new URL(request.url);
	const redirectUri = url.searchParams.get("redirect_uri");
	const state = url.searchParams.get("state") ?? crypto.randomUUID();
	const scope = url.searchParams.get("scope") ?? "openid profile email";
	const responseType = url.searchParams.get("response_type") ?? "code";

	if (!redirectUri) {
		return errorResponse(400, "invalid_request", "missing required parameter: redirect_uri");
	}

	// Build the upstream authorisation URL.
	const providerUrl = new URL(env.OIDC_PROVIDER_AUTHORIZE_URL);
	providerUrl.searchParams.set("client_id", env.OIDC_CLIENT_ID);
	providerUrl.searchParams.set("response_type", responseType);
	providerUrl.searchParams.set("redirect_uri", redirectUri);
	providerUrl.searchParams.set("scope", scope);
	providerUrl.searchParams.set("state", state);

	return Response.redirect(providerUrl.toString(), 302);
}

/**
 * GET /callback
 *
 * Handle the OIDC provider's authorisation callback.  The provider redirects
 * here with an authorisation `code` (and the original `state`).  The proxy
 * exchanges the code for tokens and returns them as JSON.
 */
async function handleCallback(request: Request, env: Env): Promise<Response> {
	const url = new URL(request.url);
	const code = url.searchParams.get("code");
	const state = url.searchParams.get("state");

	if (!code) {
		return errorResponse(400, "invalid_request", "missing required parameter: code");
	}

	// Exchange the authorisation code for tokens.
	return exchangeCode(code, state, env);
}

/**
 * POST /token
 *
 * Exchange an authorisation code (or refresh token) for tokens.  Accepts
 * `application/x-www-form-urlencoded` or `application/json` bodies.
 */
async function handleToken(request: Request, env: Env): Promise<Response> {
	const contentType = request.headers.get("content-type") ?? "";
	let grantType: string | null;
	let code: string | null;
	let redirectUri: string | null;

	if (contentType.includes("application/json")) {
		const body: Record<string, unknown> = await request.json().catch(() => ({}));
		grantType = String(body.grant_type ?? "");
		code = body.code != null ? String(body.code) : null;
		redirectUri = body.redirect_uri != null ? String(body.redirect_uri) : null;
	} else {
		const form = await request.formData().catch(() => new FormData());
		grantType = form.get("grant_type") as string | null;
		code = form.get("code") as string | null;
		redirectUri = form.get("redirect_uri") as string | null;
	}

	if (!grantType) {
		return errorResponse(400, "invalid_request", "missing required parameter: grant_type");
	}

	if (grantType === "authorization_code") {
		if (!code) {
			return errorResponse(400, "invalid_request", "missing required parameter: code");
		}
		return exchangeCode(code, null, env);
	}

	if (grantType === "refresh_token") {
		const refreshToken = contentType.includes("application/json")
			? ((await request.json().catch(() => ({}))) as Record<string, unknown>).refresh_token
			: (await request.formData().catch(() => new FormData())).get("refresh_token");

		if (!refreshToken) {
			return errorResponse(400, "invalid_request", "missing required parameter: refresh_token");
		}

		return refreshTokens(String(refreshToken), env);
	}

	return errorResponse(400, "unsupported_grant_type", `grant_type '${grantType}' is not supported`);
}

/**
 * GET /.well-known/openid-configuration
 *
 * Return an OIDC Discovery document describing the proxy endpoints.
 */
function handleWellKnown(_request: Request, env: Env): Response {
	const issuer = env.OIDC_ISSUER.replace(/\/+$/, "");
	const base = issuer;

	const config = {
		issuer,
		authorization_endpoint: `${base}/authorize`,
		token_endpoint: `${base}/token`,
		userinfo_endpoint: null,
		jwks_uri: `${base}/.well-known/jwks.json`,
		response_types_supported: ["code"],
		response_modes_supported: ["query", "fragment"],
		grant_types_supported: ["authorization_code", "refresh_token"],
		subject_types_supported: ["public"],
		id_token_signing_alg_values_supported: ["RS256"],
		scopes_supported: ["openid", "profile", "email"],
		token_endpoint_auth_methods_supported: ["client_secret_post"],
		claims_supported: ["sub", "iss", "aud", "exp", "iat"],
		code_challenge_methods_supported: [],
	};

	return new Response(JSON.stringify(config, null, 2), {
		headers: { "content-type": "application/json; charset=utf-8" },
	});
}

// ── Token exchange helpers ───────────────────────────────────────────────

/**
 * Exchange an authorisation code for tokens by calling the upstream OIDC
 * provider's token endpoint.
 */
async function exchangeCode(code: string, _state: string | null, env: Env): Promise<Response> {
	const params = new URLSearchParams();
	params.set("grant_type", "authorization_code");
	params.set("code", code);
	params.set("client_id", env.OIDC_CLIENT_ID);
	params.set("client_secret", env.OIDC_CLIENT_SECRET);

	let response: Response;
	try {
		response = await fetch(env.OIDC_PROVIDER_TOKEN_URL, {
			method: "POST",
			headers: { "content-type": "application/x-www-form-urlencoded" },
			body: params.toString(),
		});
	} catch (err) {
		const msg = err instanceof Error ? err.message : String(err);
		return errorResponse(502, "upstream_error", `failed to reach token endpoint: ${msg}`);
	}

	if (!response.ok) {
		const body = await response.text().catch(() => "unknown error");
		return errorResponse(
			response.status,
			"upstream_error",
			`token endpoint returned ${response.status}: ${body.slice(0, 512)}`,
		);
	}

	const tokens = await response.json().catch(() => null);
	if (!tokens) {
		return errorResponse(502, "upstream_error", "token endpoint returned non-JSON body");
	}

	return new Response(JSON.stringify(tokens, null, 2), {
		status: 200,
		headers: { "content-type": "application/json; charset=utf-8" },
	});
}

/**
 * Refresh an access token using a refresh token.
 */
async function refreshTokens(refreshToken: string, env: Env): Promise<Response> {
	const params = new URLSearchParams();
	params.set("grant_type", "refresh_token");
	params.set("refresh_token", refreshToken);
	params.set("client_id", env.OIDC_CLIENT_ID);
	params.set("client_secret", env.OIDC_CLIENT_SECRET);

	let response: Response;
	try {
		response = await fetch(env.OIDC_PROVIDER_TOKEN_URL, {
			method: "POST",
			headers: { "content-type": "application/x-www-form-urlencoded" },
			body: params.toString(),
		});
	} catch (err) {
		const msg = err instanceof Error ? err.message : String(err);
		return errorResponse(502, "upstream_error", `failed to reach token endpoint: ${msg}`);
	}

	if (!response.ok) {
		const body = await response.text().catch(() => "unknown error");
		return errorResponse(
			response.status,
			"upstream_error",
			`token endpoint returned ${response.status}: ${body.slice(0, 512)}`,
		);
	}

	const tokens = await response.json().catch(() => null);
	if (!tokens) {
		return errorResponse(502, "upstream_error", "token endpoint returned non-JSON body");
	}

	return new Response(JSON.stringify(tokens, null, 2), {
		status: 200,
		headers: { "content-type": "application/json; charset=utf-8" },
	});
}

// ── Worker entry point ───────────────────────────────────────────────────

export default {
	async fetch(request: Request, env: Env): Promise<Response> {
		const route = matchRoute(request);

		try {
			switch (route.kind) {
				case "authorize":
					return handleAuthorize(request, env);
				case "callback":
					return await handleCallback(request, env);
				case "token":
					return await handleToken(request, env);
				case "wellKnown":
					return handleWellKnown(request, env);
				case "notFound":
					return errorResponse(404, "not_found", `no route for ${request.method} ${new URL(request.url).pathname}`);
			}
		} catch (err) {
			const msg = err instanceof Error ? err.message : String(err);
			return errorResponse(500, "internal_error", msg);
		}
	},
};
