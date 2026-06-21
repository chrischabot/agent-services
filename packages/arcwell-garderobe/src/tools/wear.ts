import { z } from "zod";
import type { WardrobeMCP } from "../mcp";
import { findItem, queryAll, resolveItemIds } from "../db";
import { getRotation, loadRotationSlots } from "../rotation-core";
import { table, ulidLike } from "../util";

export function registerWearTools(agent: WardrobeMCP): void {
  agent.server.tool(
    "log_suggestions",
    "Log every outfit set immediately after drafting options. This feeds probabilistic availability and keeps future outfit requests honest.",
    {
      for_date: z.string(),
      n_options: z.number().int().positive(),
      context: z.string().optional(),
      options: z.array(
        z.object({
          option_no: z.number().int().positive(),
          slots: z.record(z.string(), z.string())
        })
      )
    },
    async ({ for_date, n_options, context, options }) => {
      const id = ulidLike("set_");
      const statements: D1PreparedStatement[] = [
        agent.db.prepare(
          "INSERT INTO suggestion_sets(id, for_date, n_options, context) VALUES (?, ?, ?, ?)"
        ).bind(id, for_date, n_options, context ?? null)
      ];
      for (const option of options) {
        for (const [role, idOrRef] of Object.entries(option.slots)) {
          const item = await findItem(agent.db, idOrRef);
          if (!item) throw new Error(`No item found for ${idOrRef}`);
          statements.push(
            agent.db.prepare(
              "INSERT INTO suggestion_items(set_id, option_no, role, item_id) VALUES (?, ?, ?, ?)"
            ).bind(id, option.option_no, role, item.id)
          );
        }
      }
      await agent.db.batch(statements);
      return { content: [{ type: "text", text: `Logged suggestion set ${id} with ${options.length} options.` }] };
    }
  );

  agent.server.tool(
    "confirm_wear",
    "Confirm what was actually worn. Accepts an option number, explicit item_ids, swaps, or a rotation_day to expand the planned rotation into the wear log.",
    {
      date: z.string(),
      option_no: z.number().int().positive().optional(),
      item_ids: z.array(z.string()).optional(),
      swaps: z.record(z.string(), z.string()).optional(),
      rotation_day: z.number().int().positive().optional(),
      notes: z.string().optional()
    },
    async ({ date, option_no, item_ids, swaps, rotation_day, notes }) => {
      const outfitRef = ulidLike("out_");
      const ids = new Set<string>();
      let source: "confirmed" | "rotation" = "confirmed";
      const resolveSuggestionSetIds = new Set<string>();
      if (rotation_day) {
        const rotation = await getRotation(agent.db);
        const rows = await loadRotationSlots(agent.db, rotation.id, rotation_day);
        if (rows.length === 0) throw new Error(`${rotation.name} day ${rotation_day} has no assigned slots`);
        for (const row of rows) ids.add(row.item_id);
        source = "rotation";
      }
      if (option_no) {
        const set = await agent.db.prepare(
          "SELECT id FROM suggestion_sets WHERE for_date = ? AND resolved = 0 ORDER BY created_at DESC LIMIT 1"
        )
          .bind(date)
          .first<{ id: string }>();
        if (!set) throw new Error(`No unresolved suggestion set found for ${date}`);
        const optionItems = await queryAll<{ item_id: string }>(
          agent.db,
          "SELECT item_id FROM suggestion_items WHERE set_id = ? AND option_no = ?",
          [set.id, option_no]
        );
        for (const row of optionItems) ids.add(row.item_id);
        resolveSuggestionSetIds.add(set.id);
      }
      if (item_ids?.length) {
        for (const id of await resolveItemIds(agent.db, item_ids)) ids.add(id);
      }
      if (swaps) {
        for (const id of await resolveItemIds(agent.db, Object.values(swaps))) ids.add(id);
      }
      if (ids.size === 0) throw new Error("No items were supplied or resolved for confirmation");
      const statements = [
        ...[...ids].map((itemId) =>
          agent.db.prepare(
            "INSERT INTO wear_log(id, item_id, worn_date, outfit_ref, source, notes) VALUES (?, ?, ?, ?, ?, ?)"
          ).bind(ulidLike("wear_"), itemId, date, outfitRef, source, notes ?? null)
        ),
        ...[...resolveSuggestionSetIds].map((setId) =>
          agent.db.prepare("UPDATE suggestion_sets SET resolved = 1 WHERE id = ?").bind(setId)
        ),
        ...(rotation_day
          ? [agent.db.prepare("UPDATE suggestion_sets SET resolved = 1 WHERE for_date = ? AND resolved = 0").bind(date)]
          : [])
      ];
      await agent.db.batch(statements);
      return { content: [{ type: "text", text: `Confirmed ${ids.size} ${source} items for ${date} as ${outfitRef}.` }] };
    }
  );

  agent.server.tool(
    "wear_history",
    "Return confirmed wear history for an item or date range. Estimated history remains labelled separately in outfit_pool and stats.",
    {
      item_id: z.string().optional(),
      from: z.string().optional(),
      to: z.string().optional(),
      include_estimated: z.boolean().default(true)
    },
    async ({ item_id, from, to }) => {
      let itemFilter = "";
      const bind: unknown[] = [];
      if (item_id) {
        const item = await findItem(agent.db, item_id);
        if (!item) throw new Error(`No item found for ${item_id}`);
        itemFilter = "AND wl.item_id = ?";
        bind.push(item.id);
      }
      if (from) {
        itemFilter += " AND wl.worn_date >= ?";
        bind.push(from);
      }
      if (to) {
        itemFilter += " AND wl.worn_date <= ?";
        bind.push(to);
      }
      const rows = await queryAll<Record<string, unknown>>(
        agent.db,
        `SELECT wl.worn_date, i.name, i.category, wl.source, wl.outfit_ref, wl.notes
         FROM wear_log wl
         JOIN items i ON i.id = wl.item_id
         WHERE 1=1 ${itemFilter}
         ORDER BY wl.worn_date DESC, i.category`,
        bind
      );
      return { content: [{ type: "text", text: table(rows) }] };
    }
  );

  agent.server.tool(
    "wardrobe_stats",
    "Summarise wardrobe usage: counts, spend, confirmed wears, cost-per-wear, and rotation equity for the active cycle.",
    { category: z.string().optional() },
    async ({ category }) => {
      const bind = category ? [category] : [];
      const categoryClause = category ? "WHERE i.category = ?" : "";
      const rows = await queryAll<Record<string, unknown>>(
        agent.db,
        `SELECT i.category,
                count(*) AS items,
                round(sum(coalesce(i.price, 0)), 2) AS spend_gbp,
                sum(CASE WHEN wl.id IS NULL THEN 0 ELSE 1 END) AS confirmed_wears,
                sum(CASE WHEN wl.id IS NULL THEN 1 ELSE 0 END) AS never_worn_rows
         FROM items i
         LEFT JOIN wear_log wl ON wl.item_id = i.id
         ${categoryClause}
         GROUP BY i.category
         ORDER BY i.category`,
        bind
      );
      const never = await queryAll<Record<string, unknown>>(
        agent.db,
        `SELECT i.id, i.name, i.category, i.brand, i.status
         FROM items i
         LEFT JOIN wear_log wl ON wl.item_id = i.id
         WHERE wl.id IS NULL ${category ? "AND i.category = ?" : ""}
         ORDER BY i.category, i.name
         LIMIT 30`,
        bind
      );
      const rotationEquity = await queryAll<Record<string, unknown>>(
        agent.db,
        `WITH active_rotation AS (
           SELECT id FROM rotations WHERE active = 1 LIMIT 1
         ), rotation_counts AS (
           SELECT item_id, count(*) AS rotation_days
           FROM rotation_slots
           WHERE rotation_id = (SELECT id FROM active_rotation)
           GROUP BY item_id
         )
         SELECT i.id, i.name, i.category, i.status, coalesce(rc.rotation_days, 0) AS rotation_days
         FROM items i
         LEFT JOIN rotation_counts rc ON rc.item_id = i.id
         WHERE i.deleted_at IS NULL
           AND i.status IN ('active', 'breaking_in')
           ${category ? "AND i.category = ?" : ""}
         ORDER BY rotation_days ASC, i.category, i.name
         LIMIT 40`,
        bind
      );
      return {
        content: [
          {
            type: "text",
            text: `By category\n${table(rows)}\n\nNever worn/sample\n${table(never)}\n\nRotation equity (zero/low rotation days first)\n${table(rotationEquity)}`
          }
        ]
      };
    }
  );
}
