import OAuthProvider from "@cloudflare/workers-oauth-provider";
import { AuthHandler } from "./auth";
import { exportInventoryCsv, handleAdmin } from "./admin";
import { WardrobeMCP } from "./mcp";
import type { Env } from "./types";

export { WardrobeMCP };

const provider = new OAuthProvider<Env>({
  apiHandler: WardrobeMCP.serve("/mcp"),
  apiRoute: "/mcp",
  authorizeEndpoint: "/authorize",
  clientRegistrationEndpoint: "/register",
  defaultHandler: AuthHandler,
  tokenEndpoint: "/token",
  scopesSupported: ["wardrobe.read", "wardrobe.write"],
  allowPlainPKCE: false
});

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    const url = new URL(request.url);
    if (url.pathname === "/" && request.method === "GET") {
      return Response.redirect(new URL("/admin", request.url).toString(), 302);
    }
    if (url.pathname === "/login" || url.pathname.startsWith("/admin")) {
      return await handleAdmin(request, env);
    }
    return await provider.fetch(request, env, ctx);
  },

  async scheduled(_event: ScheduledEvent, env: Env, ctx: ExecutionContext): Promise<void> {
    ctx.waitUntil(provider.purgeExpiredData(env));
    ctx.waitUntil(
      (async () => {
        const csv = await exportInventoryCsv(env);
        await env.OAUTH_KV.put(`backup:inventory:${new Date().toISOString().slice(0, 10)}`, csv);
      })()
    );
  }
};
