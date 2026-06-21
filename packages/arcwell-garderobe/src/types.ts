import type { OAuthHelpers } from "@cloudflare/workers-oauth-provider";
import type { WardrobeMCP } from "./mcp";

export type Env = {
  DB: D1Database;
  OAUTH_KV: KVNamespace;
  OAUTH_PROVIDER: OAuthHelpers;
  MCP_OBJECT: DurableObjectNamespace<WardrobeMCP>;
  WARDROBE_USER_ID: string;
  WARDROBE_USER_EMAIL: string;
  WARDROBE_LOGIN_CODE: string;
  COOKIE_SECRET: string;
};

export type AuthProps = {
  userId: string;
  email: string;
};

export type Item = {
  id: string;
  name: string;
  category: string;
  subcategory: string | null;
  colour: string | null;
  pattern: string | null;
  fabric: string | null;
  brand: string | null;
  size: string | null;
  fit_notes: string | null;
  seasons: string | null;
  temp_min_c: number | null;
  temp_max_c: number | null;
  formality: number | null;
  status: string;
  notes: string | null;
  source_detail: string | null;
  aliases: string | null;
  tags: string | null;
  price: number | null;
  currency: string | null;
  acquired_date: string | null;
  link: string | null;
  ref_code: string | null;
  quantity: number;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
};
