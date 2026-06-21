import { z } from "zod";
import type { WardrobeMCP } from "../mcp";
import { findItem, queryAll, run } from "../db";
import {
  compactDayLine,
  formatRotationDay,
  getRotation,
  loadRotationSlots,
  resolveItemForSlot,
  resolveRotationDay,
  roleCategories,
  rotationHeader,
  rotationRoles,
  setRotationDaySlots,
  warningForSlot
} from "../rotation-core";
import { table, ulidLike } from "../util";

export function registerRotationTools(agent: WardrobeMCP): void {
  agent.server.tool(
    "get_rotation",
    "Use this to inspect the active outfit rotation, a named rotation, or today's/tomorrow's planned outfit. For a single day it expands item IDs, colours, fit notes, and availability.",
    {
      name: z.string().optional(),
      day: z.union([z.number().int().positive(), z.enum(["today", "tomorrow"])]).optional()
    },
    async ({ name, day }) => {
      const rotation = await getRotation(agent.db, name);
      const resolvedDay = await resolveRotationDay(agent.db, rotation, day);
      if (resolvedDay) {
        return { content: [{ type: "text", text: await formatRotationDay(agent.db, rotation, resolvedDay) }] };
      }
      const rows = await loadRotationSlots(agent.db, rotation.id);
      const byDay = new Map<number, typeof rows>();
      for (let dayNo = 1; dayNo <= rotation.days; dayNo++) byDay.set(dayNo, []);
      for (const row of rows) byDay.set(row.day_number, [...(byDay.get(row.day_number) ?? []), row]);
      return {
        content: [
          {
            type: "text",
            text: [
              rotationHeader(rotation),
              ...[...byDay.entries()].map(([dayNo, dayRows]) => compactDayLine(dayNo, dayRows))
            ].join("\n")
          }
        ]
      };
    }
  );

  agent.server.tool(
    "set_rotation_day",
    "Use this to set or patch one day in a rotation. It resolves every id/ref/name before writing and fails the whole day if any slot is invalid.",
    {
      rotation: z.string().optional(),
      day: z.number().int().positive(),
      slots: z.record(z.string(), z.string()),
      replace: z.boolean().default(true)
    },
    async ({ rotation: rotationName, day, slots, replace }) => {
      const rotation = await getRotation(agent.db, rotationName);
      const result = await setRotationDaySlots(agent.db, rotation, day, slots, replace);
      return {
        content: [
          {
            type: "text",
            text: [result.text, result.warnings.length ? `warnings:\n${result.warnings.join("\n")}` : "warnings: none"].join(
              "\n\n"
            )
          }
        ]
      };
    }
  );

  agent.server.tool(
    "delete_rotation_day",
    "Use this to clear all planned slots for one rotation day. The day remains in the cycle and renders as unassigned.",
    { rotation: z.string().optional(), day: z.number().int().positive() },
    async ({ rotation: rotationName, day }) => {
      const rotation = await getRotation(agent.db, rotationName);
      await run(agent.db, "DELETE FROM rotation_slots WHERE rotation_id = ? AND day_number = ?", [
        rotation.id,
        day
      ]);
      return { content: [{ type: "text", text: `${rotation.name} day ${day} cleared.\n${day} · — unassigned —` }] };
    }
  );

  agent.server.tool(
    "swap_rotation_item",
    "Use this maintenance tool when an item is retired, tailored, or replaced and every occurrence in a rotation should move to another item.",
    {
      rotation: z.string().optional(),
      from: z.string(),
      to: z.string(),
      days: z.array(z.number().int().positive()).optional()
    },
    async ({ rotation: rotationName, from, to, days }) => {
      const rotation = await getRotation(agent.db, rotationName);
      const fromItem = await findItem(agent.db, from);
      if (!fromItem) throw new Error(`No item found for swap source: ${from}`);
      const toItem = await findItem(agent.db, to);
      if (!toItem) throw new Error(`No item found for swap target: ${to}`);

      const bind: unknown[] = [rotation.id, fromItem.id];
      let dayClause = "";
      if (days?.length) {
        dayClause = `AND day_number IN (${days.map(() => "?").join(",")})`;
        bind.push(...days);
      }
      const hits = await queryAll<{ day_number: number; role: string }>(
        agent.db,
        `SELECT day_number, role FROM rotation_slots
         WHERE rotation_id = ? AND item_id = ? ${dayClause}
         ORDER BY day_number, role`,
        bind
      );
      if (hits.length === 0) {
        return { content: [{ type: "text", text: `No ${fromItem.name} slots found in ${rotation.name}.` }] };
      }
      for (const hit of hits) {
        const expected = roleCategories[hit.role] ?? hit.role;
        if (toItem.category !== expected) {
          throw new Error(
            `Cannot swap ${fromItem.name} in role ${hit.role} to ${toItem.name}; target category is ${toItem.category}, expected ${expected}`
          );
        }
      }
      await agent.db.batch(
        hits.map((hit) =>
          agent.db
            .prepare("UPDATE rotation_slots SET item_id = ? WHERE rotation_id = ? AND day_number = ? AND role = ?")
            .bind(toItem.id, rotation.id, hit.day_number, hit.role)
        )
      );
      const changedDays = [...new Set(hits.map((hit) => hit.day_number))];
      const warning = warningForSlot(changedDays[0] ?? 0, hits[0]?.role ?? "slot", toItem);
      return {
        content: [
          {
            type: "text",
            text: [
              `Swapped ${fromItem.name} -> ${toItem.name} in ${rotation.name}.`,
              `days_changed: ${changedDays.join(", ") || "none"}`,
              warning ? `warnings:\n${warning}` : "warnings: none"
            ].join("\n")
          }
        ]
      };
    }
  );

  agent.server.tool(
    "manage_rotation",
    "Use this for rare structural rotation changes: create, rename, activate, set the date cursor, duplicate, archive, or delete.",
    {
      action: z.enum(["create", "rename", "activate", "set_start_date", "duplicate", "archive", "delete"]),
      name: z.string(),
      new_name: z.string().optional(),
      season: z.string().optional(),
      days: z.number().int().positive().default(28),
      start_date: z.string().optional(),
      notes: z.string().optional(),
      confirm: z.boolean().default(false)
    },
    async ({ action, name, new_name, season, days, start_date, notes, confirm }) => {
      if (action === "create") {
        await run(
          agent.db,
          "INSERT INTO rotations(id, name, season, days, notes, active, start_date) VALUES (?, ?, ?, ?, ?, 0, ?)",
          [ulidLike("rot_"), name, season ?? null, days, notes ?? null, start_date ?? null]
        );
      } else if (action === "rename") {
        if (!new_name) throw new Error("new_name is required for rename");
        await run(agent.db, "UPDATE rotations SET name = ? WHERE name = ?", [new_name, name]);
      } else if (action === "activate") {
        const rotation = await getRotation(agent.db, name);
        await agent.db.batch([
          agent.db.prepare("UPDATE rotations SET active = 0"),
          agent.db.prepare("UPDATE rotations SET active = 1 WHERE id = ?").bind(rotation.id)
        ]);
      } else if (action === "set_start_date") {
        if (!start_date) throw new Error("start_date is required for set_start_date");
        await run(agent.db, "UPDATE rotations SET start_date = ? WHERE name = ?", [start_date, name]);
      } else if (action === "duplicate") {
        if (!new_name) throw new Error("new_name is required for duplicate");
        const rotation = await getRotation(agent.db, name);
        const newId = ulidLike("rot_");
        const slots = await loadRotationSlots(agent.db, rotation.id);
        await agent.db.batch([
          agent.db
            .prepare("INSERT INTO rotations(id, name, season, days, notes, active, start_date) VALUES (?, ?, ?, ?, ?, 0, ?)")
            .bind(newId, new_name, season ?? rotation.season, rotation.days, notes ?? rotation.notes, rotation.start_date),
          ...slots.map((slot) =>
            agent.db
              .prepare("INSERT INTO rotation_slots(id, rotation_id, day_number, role, item_id) VALUES (?, ?, ?, ?, ?)")
              .bind(ulidLike("slot_"), newId, slot.day_number, slot.role, slot.item_id)
          )
        ]);
      } else if (action === "archive") {
        const rotation = await getRotation(agent.db, name);
        await run(agent.db, "UPDATE rotations SET active = 0, notes = coalesce(notes, '') || ? WHERE id = ?", [
          `\n[archived ${new Date().toISOString()}]`,
          rotation.id
        ]);
      } else if (action === "delete") {
        if (!confirm) throw new Error("delete requires confirm: true");
        const rotation = await getRotation(agent.db, name);
        if (rotation.active) throw new Error("Refusing to delete the active rotation; activate another rotation first");
        await run(agent.db, "DELETE FROM rotations WHERE id = ?", [rotation.id]);
      }

      const rows = await queryAll<Record<string, unknown>>(
        agent.db,
        "SELECT id, name, season, days, active, start_date, updated_at FROM rotations ORDER BY active DESC, name"
      );
      return { content: [{ type: "text", text: table(rows) }] };
    }
  );
}
