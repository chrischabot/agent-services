import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { McpAgent } from "agents/mcp";
import type { AuthProps, Env } from "./types";
import { registerInventoryTools } from "./tools/inventory";
import { registerPoolTools } from "./tools/pool";
import { registerWearTools } from "./tools/wear";
import { registerRotationTools } from "./tools/rotation";

export class WardrobeMCP extends McpAgent<Env, Record<string, never>, AuthProps> {
  server = new McpServer({
    name: "garderobe",
    version: "1.0.0"
  });

  get db(): D1Database {
    return this.env.DB;
  }

  async init(): Promise<void> {
    registerPoolTools(this);
    registerInventoryTools(this);
    registerWearTools(this);
    registerRotationTools(this);
  }
}
