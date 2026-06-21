import { z } from "zod";
import type { WardrobeMCP } from "../mcp";
import {
  findItem,
  insertItem,
  itemFields,
  itemSummary,
  searchItems,
  updateItemByIdOrRef
} from "../db";
import { table } from "../util";

const poolTempCategories = new Set(["shirt", "trouser", "outerwear", "knitwear", "footwear"]);

const itemPatchSchema = z
  .object({
    name: z.string().min(1).optional(),
    category: z.string().optional(),
    subcategory: z.string().nullable().optional(),
    colour: z.string().nullable().optional(),
    pattern: z.string().nullable().optional(),
    fabric: z.string().nullable().optional(),
    brand: z.string().nullable().optional(),
    size: z.string().nullable().optional(),
    fit_notes: z.string().nullable().optional(),
    seasons: z.string().nullable().optional(),
    temp_min_c: z.number().int().nullable().optional(),
    temp_max_c: z.number().int().nullable().optional(),
    formality: z.number().int().min(1).max(5).nullable().optional(),
    status: z.enum(["active", "breaking_in", "benched", "secondary", "retired"]).optional(),
    notes: z.string().nullable().optional(),
    source_detail: z.string().nullable().optional(),
    aliases: z.string().nullable().optional(),
    tags: z.array(z.string()).nullable().optional(),
    price: z.number().nullable().optional(),
    currency: z.string().nullable().optional(),
    acquired_date: z.string().nullable().optional(),
    link: z.string().nullable().optional(),
    ref_code: z.string().nullable().optional(),
    quantity: z.number().int().min(1).optional()
  })
  .strict();

type ItemPatchInput = z.infer<typeof itemPatchSchema>;

function cleanTags(tags: string[]): string[] {
  return [...new Set(tags.map((tag) => tag.trim().toLowerCase()).filter(Boolean))].sort();
}

function prepareItemPatch(input: ItemPatchInput): Record<string, unknown> {
  return {
    ...input,
    ...(input.tags !== undefined ? { tags: input.tags === null ? null : JSON.stringify(cleanTags(input.tags)) } : {})
  };
}

function missingTempWarning(item: { category: string; temp_min_c: number | null; temp_max_c: number | null }): string {
  return poolTempCategories.has(item.category) && (item.temp_min_c === null || item.temp_max_c === null)
    ? "\n⚠ no temp range — item will be flagged, not filtered, in outfit_pool."
    : "";
}

export function registerInventoryTools(agent: WardrobeMCP): void {
  agent.server.tool(
    "search_items",
    "Search wardrobe inventory. Use structured filters when known; use query for fuzzy recall such as colour names, product refs, or notes.",
    {
      query: z.string().optional(),
      category: z.string().optional(),
      subcategory: z.string().optional(),
      status: z.string().optional(),
      colour: z.string().optional(),
      brand: z.string().optional(),
      temp_c: z.number().optional(),
      formality: z.number().int().min(1).max(5).optional(),
      tags: z.array(z.string()).optional(),
      worn_within_days: z.number().int().positive().optional(),
      not_worn_within_days: z.number().int().positive().optional(),
      limit: z.number().int().positive().max(100).default(25),
      offset: z.number().int().min(0).default(0)
    },
    async (params) => {
      const rows = await searchItems(agent.db, params);
      return {
        content: [
          {
            type: "text",
            text: table(
              rows.map((item) => ({
                id: item.id,
                name: item.name,
                category: item.category,
                status: item.status,
                colour: item.colour ?? "",
                brand: item.brand ?? "",
                tags: item.tags ?? "[]",
                temp: [item.temp_min_c ?? "", item.temp_max_c ?? ""].join("-"),
                ref: item.ref_code ?? ""
              }))
            )
          }
        ]
      };
    }
  );

  agent.server.tool(
    "get_item",
    "Get a complete wardrobe item record by id, exact name, or ref code, including recent wear history and cost-per-wear.",
    { id_or_ref: z.string().min(1) },
    async ({ id_or_ref }) => {
      const item = await findItem(agent.db, id_or_ref);
      if (!item) throw new Error(`No item found for ${id_or_ref}`);
      return { content: [{ type: "text", text: await itemSummary(agent.db, item) }] };
    }
  );

  agent.server.tool(
    "add_item",
    "Add a new wardrobe item. Rejects duplicate active name+category rows unless the existing item is updated instead.",
    itemPatchSchema.extend({
      name: z.string().min(1),
      category: z.string().min(1)
    }).shape,
    async (input) => {
      const item = await insertItem(agent.db, prepareItemPatch(input));
      return { content: [{ type: "text", text: `${await itemSummary(agent.db, item)}${missingTempWarning(item)}` }] };
    }
  );

  agent.server.tool(
    "update_item",
    "Partially update a wardrobe item by id, exact name, or ref code. Returns the full post-write row.",
    {
      id_or_ref: z.string().min(1),
      patch: itemPatchSchema
    },
    async ({ id_or_ref, patch }) => {
      const item = await updateItemByIdOrRef(agent.db, id_or_ref, prepareItemPatch(patch));
      return { content: [{ type: "text", text: `${await itemSummary(agent.db, item)}${missingTempWarning(item)}` }] };
    }
  );

  agent.server.tool(
    "delete_item",
    "Delete an item by id, exact name, or ref code. Soft delete is the default; hard delete is only for import mistakes.",
    { id_or_ref: z.string().min(1), hard: z.boolean().default(false) },
    async ({ id_or_ref, hard }) => {
      const item = await findItem(agent.db, id_or_ref);
      if (!item) throw new Error(`No item found for ${id_or_ref}`);
      if (hard) {
        await agent.db.prepare("DELETE FROM items WHERE id = ?").bind(item.id).run();
        return { content: [{ type: "text", text: `Hard deleted ${item.id} ${item.name}` }] };
      }
      const retired = await updateItemByIdOrRef(agent.db, item.id, {
        deleted_at: new Date().toISOString(),
        status: "retired"
      });
      return {
        content: [
          {
            type: "text",
            text: table([
              {
                id: retired.id,
                name: retired.name,
                status: retired.status,
                deleted_at: retired.deleted_at
              }
            ], [...itemFields])
          }
        ]
      };
    }
  );
}
