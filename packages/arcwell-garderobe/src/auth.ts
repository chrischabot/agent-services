import type { AuthRequest, ClientInfo } from "@cloudflare/workers-oauth-provider";
import type { Env } from "./types";
import { escapeHtml, html } from "./util";

const sessionCookie = "__Host-garderobe_session";
const csrfCookie = "__Host-garderobe_csrf";
const maxAgeSeconds = 60 * 60 * 24 * 30;

function base64Url(bytes: ArrayBuffer | Uint8Array | string): string {
  const data =
    typeof bytes === "string"
      ? new TextEncoder().encode(bytes)
      : bytes instanceof Uint8Array
        ? bytes
        : new Uint8Array(bytes);
  let binary = "";
  for (const byte of data) binary += String.fromCharCode(byte);
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
}

async function sign(secret: string, value: string): Promise<string> {
  const key = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"]
  );
  return base64Url(await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(value)));
}

function getCookie(request: Request, name: string): string | null {
  const cookie = request.headers.get("cookie") ?? "";
  for (const part of cookie.split(";")) {
    const [key, ...rest] = part.trim().split("=");
    if (key === name) return rest.join("=");
  }
  return null;
}

async function makeSignedCookie(
  name: string,
  payload: Record<string, unknown>,
  secret: string,
  maxAge = maxAgeSeconds
): Promise<string> {
  const body = base64Url(JSON.stringify(payload));
  const sig = await sign(secret, body);
  return `${name}=${body}.${sig}; HttpOnly; Secure; Path=/; SameSite=Lax; Max-Age=${maxAge}`;
}

async function readSignedCookie<T>(
  request: Request,
  name: string,
  secret: string
): Promise<T | null> {
  const value = getCookie(request, name);
  if (!value) return null;
  const [body, sig] = value.split(".");
  if (!body || !sig) return null;
  if ((await sign(secret, body)) !== sig) return null;
  try {
    const json = atob(body.replaceAll("-", "+").replaceAll("_", "/"));
    return JSON.parse(json) as T;
  } catch {
    return null;
  }
}

async function createCsrf(secret: string): Promise<{ token: string; cookie: string }> {
  const token = crypto.randomUUID();
  return {
    token,
    cookie: await makeSignedCookie(csrfCookie, { token }, secret, 600)
  };
}

async function validateCsrf(request: Request, form: FormData, secret: string): Promise<boolean> {
  const expected = await readSignedCookie<{ token: string }>(request, csrfCookie, secret);
  return Boolean(expected?.token && expected.token === form.get("csrf"));
}

export async function hasSession(request: Request, env: Env): Promise<boolean> {
  const session = await readSignedCookie<{ userId: string; email: string; exp: number }>(
    request,
    sessionCookie,
    env.COOKIE_SECRET
  );
  return Boolean(
    session &&
      session.userId === env.WARDROBE_USER_ID &&
      session.email === env.WARDROBE_USER_EMAIL &&
      session.exp > Date.now()
  );
}

export async function createSessionCookie(env: Env): Promise<string> {
  return await makeSignedCookie(
    sessionCookie,
    {
      userId: env.WARDROBE_USER_ID,
      email: env.WARDROBE_USER_EMAIL,
      exp: Date.now() + maxAgeSeconds * 1000
    },
    env.COOKIE_SECRET
  );
}

function renderAuthorize(
  request: AuthRequest,
  client: ClientInfo | null,
  action: string,
  csrf: string,
  authed: boolean,
  error?: string
): string {
  const name = client?.clientName ?? client?.clientId ?? request.clientId;
  const scopes = request.scope.length ? request.scope.join(", ") : "default wardrobe access";
  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Authorize Garderobe</title>
  <style>
    body { margin: 0; font: 16px/1.5 system-ui, -apple-system, Segoe UI, sans-serif; background: #f8f7f2; color: #24221e; }
    main { max-width: 560px; margin: 12vh auto; padding: 0 24px; }
    h1 { font-size: 28px; line-height: 1.1; margin: 0 0 18px; }
    .panel { border: 1px solid #d8d1c2; background: #fffefa; border-radius: 8px; padding: 22px; box-shadow: 0 18px 50px rgb(42 35 20 / 8%); }
    label { display: block; font-size: 13px; font-weight: 700; margin: 18px 0 6px; }
    input { width: 100%; box-sizing: border-box; padding: 12px; border: 1px solid #bcb5a7; border-radius: 6px; font: inherit; }
    button { margin-top: 18px; padding: 10px 14px; border: 0; border-radius: 6px; background: #1c5f52; color: white; font-weight: 700; cursor: pointer; }
    .muted { color: #696156; }
    .error { color: #9c2f23; font-weight: 700; }
  </style>
</head>
<body>
  <main>
    <h1>Authorize Garderobe</h1>
    <div class="panel">
      ${error ? `<p class="error">${escapeHtml(error)}</p>` : ""}
      <p><strong>${escapeHtml(name)}</strong> wants access to your private wardrobe inventory MCP server.</p>
      <p class="muted">Scopes: ${escapeHtml(scopes)}</p>
      <form method="post" action="${escapeHtml(action)}">
        <input type="hidden" name="csrf" value="${escapeHtml(csrf)}">
        ${authed ? "" : `<label for="code">Wardrobe login code</label><input id="code" name="code" type="password" autocomplete="current-password" required>`}
        <button type="submit">Approve connector</button>
      </form>
    </div>
  </main>
</body>
</html>`;
}

export const AuthHandler: ExportedHandler<Env> = {
  async fetch(request, env) {
    const url = new URL(request.url);

    if (url.pathname === "/authorize") {
      let oauthReqInfo: AuthRequest;
      try {
        oauthReqInfo = await env.OAUTH_PROVIDER.parseAuthRequest(request);
      } catch (error) {
        console.warn("Invalid OAuth authorize request", String(error));
        return html("Invalid OAuth authorization request. Start again from Claude Connectors.", {
          status: 400
        });
      }
      const client = await env.OAUTH_PROVIDER.lookupClient(oauthReqInfo.clientId);
      const authed = await hasSession(request, env);
      const action = `${url.pathname}${url.search}`;

      if (request.method === "GET") {
        const csrf = await createCsrf(env.COOKIE_SECRET);
        return html(renderAuthorize(oauthReqInfo, client, action, csrf.token, authed), {
          headers: { "set-cookie": csrf.cookie }
        });
      }

      if (request.method !== "POST") return new Response("Method not allowed", { status: 405 });
      const form = await request.formData();
      if (!(await validateCsrf(request, form, env.COOKIE_SECRET))) {
        const csrf = await createCsrf(env.COOKIE_SECRET);
        return html(renderAuthorize(oauthReqInfo, client, action, csrf.token, authed, "Session expired. Try again."), {
          status: 400,
          headers: { "set-cookie": csrf.cookie }
        });
      }

      let setCookie = "";
      if (!authed) {
        const code = String(form.get("code") ?? "");
        if (!env.WARDROBE_LOGIN_CODE || code !== env.WARDROBE_LOGIN_CODE) {
          const csrf = await createCsrf(env.COOKIE_SECRET);
          return html(renderAuthorize(oauthReqInfo, client, action, csrf.token, false, "Invalid login code."), {
            status: 401,
            headers: { "set-cookie": csrf.cookie }
          });
        }
        setCookie = await createSessionCookie(env);
      }

      let redirectTo: string;
      try {
        ({ redirectTo } = await env.OAUTH_PROVIDER.completeAuthorization({
          request: oauthReqInfo,
          userId: env.WARDROBE_USER_ID,
          metadata: {
            email: env.WARDROBE_USER_EMAIL,
            client: client?.clientName ?? oauthReqInfo.clientId
          },
          scope: oauthReqInfo.scope,
          props: {
            userId: env.WARDROBE_USER_ID,
            email: env.WARDROBE_USER_EMAIL
          }
        }));
      } catch (error) {
        console.warn("OAuth authorization failed", String(error));
        const csrf = await createCsrf(env.COOKIE_SECRET);
        return html(renderAuthorize(oauthReqInfo, client, action, csrf.token, authed, "Authorization failed. Start again from Claude Connectors."), {
          status: 400,
          headers: { "set-cookie": csrf.cookie }
        });
      }
      console.log(
        "OAuth authorization approved",
        JSON.stringify({
          clientId: oauthReqInfo.clientId,
          redirectOrigin: new URL(redirectTo).origin
        })
      );
      return html(`<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta http-equiv="refresh" content="0; url=${escapeHtml(redirectTo)}">
  <title>Returning to Claude</title>
  <style>
    body { margin: 0; font: 16px/1.5 system-ui, -apple-system, Segoe UI, sans-serif; background: #f8f7f2; color: #24221e; }
    main { max-width: 560px; margin: 12vh auto; padding: 0 24px; }
    a { color: #1c5f52; font-weight: 700; }
  </style>
</head>
<body>
  <main>
    <h1>Returning to Claude...</h1>
    <p>If this window does not move on automatically, <a href="${escapeHtml(redirectTo)}">continue to Claude</a>.</p>
  </main>
</body>
</html>`, {
        headers: {
          ...(setCookie ? { "set-cookie": setCookie } : {}),
          refresh: `0; url=${redirectTo}`
        }
      });
    }

    return new Response("Not found", { status: 404 });
  }
};

export async function requireAdmin(request: Request, env: Env): Promise<Response | null> {
  if (await hasSession(request, env)) return null;
  return html(`<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Garderobe Login</title>
<style>body{font:16px system-ui;margin:0;background:#f8f7f2;color:#24221e}main{max-width:420px;margin:14vh auto;padding:0 24px}.panel{background:#fffefa;border:1px solid #d8d1c2;border-radius:8px;padding:22px}input{width:100%;box-sizing:border-box;padding:12px}button{margin-top:14px;padding:10px 14px;background:#1c5f52;color:white;border:0;border-radius:6px;font-weight:700}</style></head>
<body><main><h1>Garderobe</h1><div class="panel"><form method="post" action="/login"><label>Login code</label><input name="code" type="password" required><button>Sign in</button></form></div></main></body></html>`, {
    status: 401
  });
}

export async function handleLogin(request: Request, env: Env): Promise<Response> {
  const form = await request.formData();
  if (String(form.get("code") ?? "") !== env.WARDROBE_LOGIN_CODE) {
    return html("Invalid login code", { status: 401 });
  }
  return new Response(null, {
    status: 302,
    headers: {
      location: new URL("/admin", request.url).toString(),
      "set-cookie": await createSessionCookie(env)
    }
  });
}
