import type { Item } from "./types";
import { compact, table, ulidLike } from "./util";

export type SearchParams = {
  query?: string;
  category?: string;
  subcategory?: string;
  status?: string;
  colour?: string;
  brand?: string;
  tags?: string[];
  temp_c?: number;
  formality?: number;
  worn_within_days?: number;
  not_worn_within_days?: number;
  limit?: number;
  offset?: number;
};

export type Availability = {
  itemId: string;
  expectedWears: number;
  confirmedWears: number;
  estimatedWears: number;
  pAvailable: number;
  label: string;
};

export const itemFields = [
  "id",
  "name",
  "category",
  "subcategory",
  "colour",
  "pattern",
  "fabric",
  "brand",
  "size",
  "fit_notes",
  "seasons",
  "temp_min_c",
  "temp_max_c",
  "formality",
  "status",
  "notes",
  "source_detail",
  "aliases",
  "tags",
  "price",
  "currency",
  "acquired_date",
  "link",
  "ref_code",
  "quantity",
  "created_at",
  "updated_at",
  "deleted_at"
] as const;

export async function queryAll<T>(
  db: D1Database,
  sql: string,
  params: unknown[] = []
): Promise<T[]> {
  const result = await db.prepare(sql).bind(...params).all<T>();
  return result.results ?? [];
}

export async function queryFirst<T>(
  db: D1Database,
  sql: string,
  params: unknown[] = []
): Promise<T | null> {
  return await db.prepare(sql).bind(...params).first<T>();
}

export async function run(
  db: D1Database,
  sql: string,
  params: unknown[] = []
): Promise<D1Result> {
  return await db.prepare(sql).bind(...params).run();
}

export function normalizeCategory(value: string): string {
  const normalized = value.trim().toLowerCase();
  const map: Record<string, string> = {
    shirts: "shirt",
    shirt: "shirt",
    trousers: "trouser",
    trouser: "trouser",
    pants: "trouser",
    outerwear: "outerwear",
    jacket: "outerwear",
    knitwear: "knitwear",
    knit: "knitwear",
    footwear: "footwear",
    shoes: "footwear",
    shoe: "footwear",
    socks: "sock",
    sock: "sock",
    accessories: "accessory",
    accessory: "accessory"
  };
  return map[normalized] ?? normalized;
}

function ftsQuery(input: string): string {
  const tokens = input
    .toLowerCase()
    .match(/[\p{L}\p{N}]+/gu)
    ?.filter((token) => token.length > 1)
    .slice(0, 10);
  return tokens?.map((token) => `"${token.replaceAll('"', '""')}"`).join(" ") ?? "";
}

export async function findItem(db: D1Database, idOrRef: string): Promise<Item | null> {
  const key = idOrRef.trim();
  return await queryFirst<Item>(
    db,
    `SELECT ${itemFields.join(", ")} FROM items
     WHERE deleted_at IS NULL AND (id = ? OR ref_code = ? OR lower(name) = lower(?))
     LIMIT 1`,
    [key, key, key]
  );
}

export async function searchItems(db: D1Database, params: SearchParams): Promise<Item[]> {
  const where = ["items.deleted_at IS NULL"];
  const bind: unknown[] = [];
  const joins: string[] = [];
  let order = "items.category, items.name";

  if (params.query?.trim()) {
    const match = ftsQuery(params.query);
    if (match) {
      joins.push("JOIN items_fts ON items_fts.rowid = items.rowid");
      where.push("items_fts MATCH ?");
      bind.push(match);
      order = "bm25(items_fts), items.category, items.name";
    } else {
      where.push("(lower(items.name) LIKE ? OR lower(items.notes) LIKE ?)");
      bind.push(`%${params.query.toLowerCase()}%`, `%${params.query.toLowerCase()}%`);
    }
  }
  if (params.category) {
    where.push("items.category = ?");
    bind.push(normalizeCategory(params.category));
  }
  if (params.subcategory) {
    where.push("lower(items.subcategory) LIKE ?");
    bind.push(`%${params.subcategory.toLowerCase()}%`);
  }
  if (params.status) {
    where.push("items.status = ?");
    bind.push(params.status);
  }
  if (params.colour) {
    where.push("lower(coalesce(items.colour, '')) LIKE ?");
    bind.push(`%${params.colour.toLowerCase()}%`);
  }
  if (params.brand) {
    where.push("lower(coalesce(items.brand, '')) LIKE ?");
    bind.push(`%${params.brand.toLowerCase()}%`);
  }
  if (params.tags?.length) {
    for (const tag of params.tags) {
      where.push("lower(coalesce(items.tags, '')) LIKE ?");
      bind.push(`%"${tag.toLowerCase()}"%`);
    }
  }
  if (typeof params.temp_c === "number") {
    where.push("(items.temp_min_c IS NULL OR items.temp_min_c <= ?)");
    where.push("(items.temp_max_c IS NULL OR items.temp_max_c >= ?)");
    bind.push(params.temp_c, params.temp_c);
  }
  if (typeof params.formality === "number") {
    where.push("(items.formality IS NULL OR items.formality = ?)");
    bind.push(params.formality);
  }
  if (typeof params.worn_within_days === "number") {
    where.push(`EXISTS (
      SELECT 1 FROM wear_log
      WHERE wear_log.item_id = items.id
      AND wear_log.worn_date >= date('now', ?)
    )`);
    bind.push(`-${params.worn_within_days} days`);
  }
  if (typeof params.not_worn_within_days === "number") {
    where.push(`NOT EXISTS (
      SELECT 1 FROM wear_log
      WHERE wear_log.item_id = items.id
      AND wear_log.worn_date >= date('now', ?)
    )`);
    bind.push(`-${params.not_worn_within_days} days`);
  }

  const limit = Math.min(Math.max(params.limit ?? 25, 1), 100);
  const offset = Math.max(params.offset ?? 0, 0);
  bind.push(limit, offset);

  return await queryAll<Item>(
    db,
    `SELECT ${itemFields.map((f) => `items.${f}`).join(", ")}
     FROM items
     ${joins.join("\n")}
     WHERE ${where.join(" AND ")}
     ORDER BY ${order}
     LIMIT ? OFFSET ?`,
    bind
  );
}

export async function itemSummary(db: D1Database, item: Item): Promise<string> {
  const wearCount = await queryFirst<{ count: number }>(
    db,
    "SELECT count(*) AS count FROM wear_log WHERE item_id = ?",
    [item.id]
  );
  const recent = await queryAll<{ worn_date: string; source: string; notes: string | null }>(
    db,
    "SELECT worn_date, source, notes FROM wear_log WHERE item_id = ? ORDER BY worn_date DESC LIMIT 5",
    [item.id]
  );
  const costPerWear =
    item.price && wearCount?.count ? `GBP ${(item.price / wearCount.count).toFixed(2)}` : "n/a";
  return [
    table([
      {
        id: item.id,
        name: item.name,
        category: item.category,
        status: item.status,
        colour: item.colour ?? "",
        brand: item.brand ?? "",
        size: item.size ?? "",
        temp: compact([item.temp_min_c, item.temp_max_c]).join("-") || "",
        ref: item.ref_code ?? ""
      }
    ]),
    "",
    `Notes: ${item.notes ?? "n/a"}`,
    `Fit: ${item.fit_notes ?? "n/a"}`,
    `Tags: ${item.tags ?? "[]"}`,
    `Source: ${item.source_detail ?? "n/a"}`,
    `Wear count: ${wearCount?.count ?? 0}; cost per confirmed wear: ${costPerWear}`,
    recent.length ? `Recent wears:\n${table(recent)}` : "Recent wears: none"
  ].join("\n");
}

export async function calculateAvailability(
  db: D1Database,
  items: Item[],
  dateIso: string,
  alpha = 0.8
): Promise<Map<string, Availability>> {
  if (items.length === 0) return new Map();
  const itemIds = new Set(items.map((item) => item.id));
  const configs = await queryAll<{
    category: string;
    launders: number;
    cooldown_days: number;
    wears_per_cycle: number;
  }>(db, "SELECT category, launders, cooldown_days, wears_per_cycle FROM category_config");
  const configByCategory = new Map(configs.map((config) => [config.category, config]));
  const maxWindow = Math.max(...configs.map((config) => config.cooldown_days), 7);
  const since = new Date(`${dateIso}T00:00:00Z`);
  since.setUTCDate(since.getUTCDate() - maxWindow);
  const sinceIso = since.toISOString().slice(0, 10);

  const confirmedRows = await queryAll<{ item_id: string; worn_date: string }>(
    db,
    `SELECT item_id, worn_date FROM wear_log
     WHERE worn_date BETWEEN ? AND ?
     AND item_id IN (${items.map(() => "?").join(",")})`,
    [sinceIso, dateIso, ...items.map((item) => item.id)]
  );
  const suggestionRows = await queryAll<{
    item_id: string;
    for_date: string;
    n_options: number;
    appearances: number;
  }>(
    db,
    `SELECT si.item_id, ss.for_date, ss.n_options, count(DISTINCT si.option_no) AS appearances
     FROM suggestion_items si
     JOIN suggestion_sets ss ON ss.id = si.set_id
     WHERE ss.resolved = 0
       AND ss.for_date BETWEEN ? AND ?
       AND si.item_id IN (${items.map(() => "?").join(",")})
     GROUP BY si.item_id, ss.id`,
    [sinceIso, dateIso, ...items.map((item) => item.id)]
  );

  const map = new Map<string, Availability>();
  for (const item of items) {
    const config = configByCategory.get(item.category) ?? {
      launders: 0,
      cooldown_days: 1,
      wears_per_cycle: 1
    };
    const itemSince = new Date(`${dateIso}T00:00:00Z`);
    itemSince.setUTCDate(itemSince.getUTCDate() - config.cooldown_days);
    const itemSinceIso = itemSince.toISOString().slice(0, 10);
    const confirmedWears = confirmedRows.filter(
      (row) => row.item_id === item.id && row.worn_date >= itemSinceIso
    ).length;
    const estimatedWears = suggestionRows
      .filter((row) => row.item_id === item.id && row.for_date >= itemSinceIso)
      .reduce((sum, row) => sum + alpha * (row.appearances / Math.max(row.n_options, 1)), 0);
    const expectedWears = confirmedWears + estimatedWears;
    const quantity = Math.max(item.quantity ?? 1, 1);
    let pAvailable = 1;
    let label = "1.00";
    if (config.launders) {
      const unitsInWash = Math.min(quantity, expectedWears / Math.max(config.wears_per_cycle, 1));
      const availableUnits = Math.max(quantity - unitsInWash, 0);
      pAvailable = Math.max(Math.min(availableUnits / quantity, 1), 0);
      label = quantity > 1 ? `${availableUnits.toFixed(1)}/${quantity}` : pAvailable.toFixed(2);
    } else {
      pAvailable = expectedWears > 0 ? 0.5 : 1;
      label = expectedWears > 0 ? "recently worn" : "1.00";
    }
    if (!itemIds.has(item.id)) continue;
    map.set(item.id, {
      itemId: item.id,
      expectedWears,
      confirmedWears,
      estimatedWears,
      pAvailable,
      label
    });
  }
  return map;
}

export async function insertItem(db: D1Database, input: Partial<Item>): Promise<Item> {
  const category = input.category ? normalizeCategory(input.category) : input.category;
  const duplicate = await queryFirst<Item>(
    db,
    `SELECT ${itemFields.join(", ")} FROM items
     WHERE deleted_at IS NULL AND lower(name) = lower(?) AND category = ?
     LIMIT 1`,
    [input.name, category]
  );
  if (duplicate) {
    throw new Error(`Duplicate active item: ${duplicate.id} ${duplicate.name}`);
  }
  const id = input.id ?? ulidLike("itm_");
  const now = new Date().toISOString();
  const values: Partial<Item> = {
    id,
    currency: "GBP",
    quantity: 1,
    status: "active",
    created_at: now,
    updated_at: now,
    ...input,
    category
  };
  const fields = itemFields.filter((field) => field in values);
  await run(
    db,
    `INSERT INTO items (${fields.join(", ")}) VALUES (${fields.map(() => "?").join(", ")})`,
    fields.map((field) => values[field])
  );
  const created = await findItem(db, id);
  if (!created) throw new Error("Item insert succeeded but row could not be reloaded");
  return created;
}

export async function updateItemByIdOrRef(
  db: D1Database,
  idOrRef: string,
  patch: Record<string, unknown>
): Promise<Item> {
  const current = await findItem(db, idOrRef);
  if (!current) throw new Error(`No item found for ${idOrRef}`);
  const nextName = typeof patch.name === "string" ? patch.name : current.name;
  const nextCategory = typeof patch.category === "string" ? normalizeCategory(patch.category) : current.category;
  const duplicate = await queryFirst<Item>(
    db,
    `SELECT ${itemFields.join(", ")} FROM items
     WHERE deleted_at IS NULL
       AND id != ?
       AND lower(name) = lower(?)
       AND category = ?
     LIMIT 1`,
    [current.id, nextName, nextCategory]
  );
  if (duplicate) {
    throw new Error(`Duplicate active item: ${duplicate.id} ${duplicate.name}`);
  }
  if (typeof patch.category === "string") patch.category = nextCategory;
  const allowed = new Set(itemFields.filter((field) => !["id", "created_at", "updated_at"].includes(field)));
  const entries = Object.entries(patch).filter(([key]) => allowed.has(key as (typeof itemFields)[number]));
  if (entries.length === 0) throw new Error("Patch did not contain any writable item fields");
  entries.push(["updated_at", new Date().toISOString()]);
  await run(
    db,
    `UPDATE items SET ${entries.map(([key]) => `${key} = ?`).join(", ")} WHERE id = ?`,
    [...entries.map(([, value]) => value), current.id]
  );
  const updated = await findItem(db, current.id);
  if (!updated) throw new Error("Item update succeeded but row could not be reloaded");
  return updated;
}

export async function resolveItemIds(db: D1Database, idsOrRefs: string[]): Promise<string[]> {
  const ids: string[] = [];
  for (const idOrRef of idsOrRefs) {
    const item = await findItem(db, idOrRef);
    if (!item) throw new Error(`No item found for ${idOrRef}`);
    ids.push(item.id);
  }
  return ids;
}
