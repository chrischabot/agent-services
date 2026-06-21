import { z } from "zod";
import type { Item } from "../types";
import type { WardrobeMCP } from "../mcp";
import { calculateAvailability, queryAll } from "../db";
import { isoDate, roleCategories, rotationMarkers } from "../rotation-core";
import { table } from "../util";

const defaultRoles = ["outerwear", "shirt", "knitwear", "trouser", "sock", "footwear", "accessory"];
const servedTempCategories = new Set(["outerwear", "shirt", "knitwear", "trouser", "sock", "footwear", "accessory"]);

type Condition = "rain" | "heavy_rain" | "wind";

type WearEquity = {
  last_worn: string;
  wears_90d: string;
};

function parseTags(item: Item): string[] {
  if (!item.tags) return [];
  try {
    const parsed = JSON.parse(item.tags) as unknown;
    return Array.isArray(parsed)
      ? parsed.filter((tag): tag is string => typeof tag === "string").map((tag) => tag.toLowerCase())
      : [];
  } catch {
    return [];
  }
}

function hasAnyTag(item: Item, tags: Set<string>): boolean {
  if (tags.size === 0) return false;
  return parseTags(item).some((tag) => tags.has(tag));
}

function formatExpected(value: number): string {
  const rounded = Math.round(value * 10) / 10;
  return Number.isInteger(rounded) ? rounded.toFixed(0) : rounded.toFixed(1);
}

function formatRole(role: string): string {
  return role === "knitwear" ? "KNITWEAR (standalone top or midlayer)" : role.toUpperCase();
}

async function configWarnings(db: D1Database): Promise<string[]> {
  const missing = await queryAll<{ category: string }>(
    db,
    `SELECT DISTINCT i.category
     FROM items i
     LEFT JOIN category_config cc ON cc.category = i.category
     WHERE i.deleted_at IS NULL
       AND cc.category IS NULL
     ORDER BY i.category`
  );
  return missing.length
    ? [`⚠ missing category_config for: ${missing.map((row) => row.category).join(", ")}`]
    : [];
}

async function loadPoolCandidates(
  db: D1Database,
  statuses: string[],
  tempC: number,
  formality?: number
): Promise<Item[]> {
  if (statuses.length === 0) return [];
  const bind: unknown[] = [...statuses, tempC, tempC];
  if (typeof formality === "number") bind.push(formality);
  return await queryAll<Item>(
    db,
    `SELECT *
     FROM items
     WHERE deleted_at IS NULL
       AND status IN (${statuses.map(() => "?").join(",")})
       AND (temp_min_c IS NULL OR temp_min_c <= ?)
       AND (temp_max_c IS NULL OR temp_max_c >= ?)
       ${typeof formality === "number" ? "AND (formality IS NULL OR formality = ?)" : ""}
     ORDER BY category, name`,
    bind
  );
}

async function wearEquity(db: D1Database, items: Item[], dateIso: string): Promise<Map<string, WearEquity>> {
  const equity = new Map<string, WearEquity>();
  if (items.length === 0) return equity;
  const ids = items.map((item) => item.id);
  const placeholders = ids.map(() => "?").join(",");
  const since = new Date(`${dateIso}T00:00:00Z`);
  since.setUTCDate(since.getUTCDate() - 90);
  const sinceIso = since.toISOString().slice(0, 10);
  const confirmed = await queryAll<{ item_id: string; last_worn: string | null; wears_90d: number }>(
    db,
    `SELECT item_id,
            max(worn_date) AS last_worn,
            sum(CASE WHEN worn_date >= ? AND worn_date <= ? THEN 1 ELSE 0 END) AS wears_90d
     FROM wear_log
     WHERE item_id IN (${placeholders})
     GROUP BY item_id`,
    [sinceIso, dateIso, ...ids]
  );
  const expected = await queryAll<{
    item_id: string;
    latest_expected: string | null;
    expected_90d: number;
  }>(
    db,
    `SELECT si.item_id,
            max(ss.for_date) AS latest_expected,
            sum(CASE WHEN ss.for_date >= ? AND ss.for_date <= ?
                THEN 0.8 * (1.0 / max(ss.n_options, 1))
                ELSE 0
            END) AS expected_90d
     FROM suggestion_items si
     JOIN suggestion_sets ss ON ss.id = si.set_id
     WHERE ss.resolved = 0
       AND si.item_id IN (${placeholders})
     GROUP BY si.item_id`,
    [sinceIso, dateIso, ...ids]
  );
  const confirmedById = new Map(confirmed.map((row) => [row.item_id, row]));
  const expectedById = new Map(expected.map((row) => [row.item_id, row]));
  for (const item of items) {
    const confirmedRow = confirmedById.get(item.id);
    const expectedRow = expectedById.get(item.id);
    const confirmedCount = confirmedRow?.wears_90d ?? 0;
    const expectedCount = expectedRow?.expected_90d ?? 0;
    const last_worn = confirmedRow?.last_worn
      ? confirmedRow.last_worn
      : expectedRow?.latest_expected
        ? `~${expectedRow.latest_expected}`
        : "never";
    equity.set(item.id, {
      last_worn,
      wears_90d: expectedCount > 0
        ? `${confirmedCount}+${formatExpected(expectedCount)}e`
        : String(confirmedCount)
    });
  }
  return equity;
}

export function registerPoolTools(agent: WardrobeMCP): void {
  agent.server.tool(
    "outfit_pool",
    "Use this before drafting any outfit request. It audits the wardrobe by role for a date/temperature, annotates availability, wear equity, and rotation context. p_avail is a probability for qty 1, or available/total for qty > 1. NULL temp ranges are flagged as ⚠ no temp range rather than hidden.",
    {
      temp_c: z.number(),
      date: z.string().optional(),
      min_p_available: z.number().min(0).max(1).default(0),
      include_statuses: z
        .array(z.enum(["active", "breaking_in", "benched", "secondary", "retired"]))
        .default(["active", "breaking_in"]),
      roles: z.array(z.string()).optional(),
      exclude_ids: z.array(z.string()).default([]),
      exclude_subcategories: z.array(z.string()).default([]),
      exclude_tags: z.array(z.string()).default([]),
      conditions: z.array(z.enum(["rain", "heavy_rain", "wind"])).default([]),
      formality: z.number().int().min(1).max(5).optional(),
      rotation_context: z.boolean().default(true)
    },
    async ({
      temp_c,
      date,
      min_p_available,
      include_statuses,
      roles,
      exclude_ids,
      exclude_subcategories,
      exclude_tags,
      conditions,
      formality,
      rotation_context
    }) => {
      const targetDate = isoDate(date);
      const wantedRoles = roles?.length ? roles : defaultRoles;
      const allCandidates = await loadPoolCandidates(agent.db, include_statuses, temp_c, formality);
      const seen = new Map(allCandidates.map((item) => [item.id, item]));
      const excludedIds = new Set(exclude_ids);
      const excludedSubcategories = new Set(exclude_subcategories.map((value) => value.toLowerCase()));
      const excludedTags = new Set(exclude_tags.map((value) => value.toLowerCase()));
      const deduped = [...seen.values()];
      const excludedByRequest = deduped.filter(
        (item) =>
          excludedIds.has(item.id) ||
          (item.subcategory ? excludedSubcategories.has(item.subcategory.toLowerCase()) : false) ||
          hasAnyTag(item, excludedTags)
      );
      const candidates = deduped.filter((item) => !excludedByRequest.some((excluded) => excluded.id === item.id));
      const availability = await calculateAvailability(agent.db, candidates, targetDate);
      const equity = await wearEquity(agent.db, candidates, targetDate);

      const benchedCount = (await queryAll<{ count: number }>(
        agent.db,
        "SELECT count(*) AS count FROM items WHERE deleted_at IS NULL AND status = 'benched'"
      ))[0]?.count ?? 0;
      const outOfTempCount = (await queryAll<{ count: number }>(
        agent.db,
        `SELECT count(*) AS count
         FROM items
         WHERE deleted_at IS NULL
           AND status IN (${include_statuses.map(() => "?").join(",")})
           AND ((temp_min_c IS NOT NULL AND temp_min_c > ?)
             OR (temp_max_c IS NOT NULL AND temp_max_c < ?))`,
        [...include_statuses, temp_c, temp_c]
      ))[0]?.count ?? 0;
      const missingTempCount = candidates.filter(
        (item) => servedTempCategories.has(item.category) && (item.temp_min_c === null || item.temp_max_c === null)
      ).length;
      const lowClean = candidates.filter((item) => (availability.get(item.id)?.pAvailable ?? 1) < 0.5)
        .length;
      const excludedByMin = candidates.filter(
        (item) => (availability.get(item.id)?.pAvailable ?? 1) < min_p_available
      ).length;
      const rotation = rotation_context
        ? await rotationMarkers(agent.db, targetDate, 2)
        : { header: [] as string[], markers: new Map<string, string[]>() };

      const sections: string[] = [
        [
          `Audit for ${targetDate}, ${temp_c}C: ${benchedCount} benched; ${outOfTempCount} active/breaking_in out of temp range; ${excludedByRequest.length} excluded by request; ${missingTempCount} items missing temp range — set via update_item; ${lowClean} items <50% likely clean; ${excludedByMin} below requested availability threshold.`,
          ...rotation.header,
          ...(await configWarnings(agent.db))
        ].join("\n")
      ];
      const rainy = conditions.some((condition: Condition) => condition === "rain" || condition === "heavy_rain");

      for (const role of wantedRoles) {
        const category = roleCategories[role] ?? role;
        const rows = candidates
          .filter((item) => item.category === category)
          .map((item) => {
            const avail = availability.get(item.id);
            const tags = parseTags(item);
            const rainOk = rainy && ["outerwear", "footwear"].includes(category) && tags.includes("rain_ok");
            const rainAverse = rainy && ["outerwear", "footwear"].includes(category) && tags.includes("rain_averse");
            const flags = [
              item.temp_min_c === null || item.temp_max_c === null ? "⚠ no temp range" : "",
              rainOk ? "☂" : "",
              rainAverse ? "⚠ rain_averse" : ""
            ].filter(Boolean);
            const rowEquity = equity.get(item.id);
            return {
              id: item.id,
              name: item.name,
              colour: item.colour ?? "",
              subcategory: item.subcategory ?? "",
              fit: item.fit_notes ?? "",
              status: item.status,
              p_avail: avail?.label ?? "1.00",
              last_worn: rowEquity?.last_worn ?? "never",
              wears_90d: rowEquity?.wears_90d ?? "0",
              rotation: rotation.markers.get(item.id)?.map((marker) => `◦${marker}`).join(" ") ?? "",
              flags: flags.join(" "),
              score: avail?.pAvailable ?? 1
            };
          })
          .filter((row) => row.score >= min_p_available)
          .sort((a, b) => {
            const aRain = a.flags.includes("☂") ? 1 : 0;
            const bRain = b.flags.includes("☂") ? 1 : 0;
            return bRain - aRain || b.score - a.score || a.name.localeCompare(b.name);
          });
        sections.push(`\n${formatRole(role)}\n${table(rows.map(({ score: _score, ...row }) => row))}`);
      }

      return { content: [{ type: "text", text: sections.join("\n") }] };
    }
  );
}
