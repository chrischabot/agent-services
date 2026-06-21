import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { dirname, resolve } from "node:path";
import { parse } from "csv-parse/sync";

const inputPath = process.argv[2];
if (!inputPath) {
  throw new Error("Usage: npm run import:csv -- /path/to/wardrobe.csv");
}

type CsvRow = string[];

const rows = parse(readFileSync(inputPath), {
  bom: true,
  relax_column_count: true,
  skip_empty_lines: true
}) as CsvRow[];

const headerIndex = rows.findIndex((row) => row[0] === "Category" && row[1] === "Item");
if (headerIndex < 0) throw new Error("Could not find wardrobe CSV header row");
const dataRows = rows.slice(headerIndex + 1).filter((row) => row[0] && row[1]);

const categoryMap: Record<string, string> = {
  Footwear: "footwear",
  Trouser: "trouser",
  Outerwear: "outerwear",
  Knitwear: "knitwear",
  Shirt: "shirt",
  Sock: "sock",
  Accessory: "accessory"
};

function sql(value: unknown): string {
  if (value === null || value === undefined || value === "") return "NULL";
  if (typeof value === "number") return Number.isFinite(value) ? String(value) : "NULL";
  return `'${String(value).replaceAll("'", "''")}'`;
}

function idFor(row: CsvRow, index: number): string {
  const hash = createHash("sha1").update(`${index}:${row.join("|")}`).digest("hex").slice(0, 18);
  return `itm_${hash}`;
}

function parseStatus(raw: string): { status: string; fitNotes: string | null } {
  const text = raw.trim();
  const parenthetical = text.match(/\((.*)\)/)?.[1] ?? null;
  const base = text.replace(/\s*\(.*\)\s*/, "").toLowerCase();
  const status =
    base === "breaking in"
      ? "breaking_in"
      : base === "benched"
        ? "benched"
        : base === "occasional" || base === "secondary/layering" || base === "active layering tier"
          ? "secondary"
          : "active";
  return { status, fitNotes: parenthetical };
}

function parseColourPattern(raw: string): { colour: string | null; pattern: string | null } {
  const value = raw.trim();
  if (!value) return { colour: null, pattern: null };
  const lower = value.toLowerCase();
  const patterns = ["stripe", "plaid", "check", "herringbone", "houndstooth", "pattern"];
  const pattern = patterns.find((candidate) => lower.includes(candidate)) ?? null;
  return { colour: value, pattern };
}

function inferTemp(raw: string): { min: number | null; max: number | null } {
  const text = raw.trim();
  const range = text.match(/(-?\d+)\s*-\s*(-?\d+)\s*°?C/i);
  if (range) return { min: Number(range[1]), max: Number(range[2]) };
  const to = text.match(/To\s+(-?\d+)\s*°?C/i);
  if (to) return { min: -5, max: Number(to[1]) };
  const lower = text.toLowerCase();
  if (!text || text === "-") return { min: null, max: null };
  if (lower.includes("30")) return { min: 30, max: 45 };
  if (lower.includes("hot")) return { min: 24, max: 40 };
  if (lower.includes("warm-weather")) return { min: 18, max: 34 };
  if (lower.includes("warm-leaning")) return { min: 14, max: 28 };
  if (lower === "warm") return { min: 16, max: 28 };
  if (lower.includes("transitional")) return { min: 8, max: 22 };
  if (lower.includes("all-but-coldest")) return { min: 5, max: 30 };
  if (lower.includes("cool/cold")) return { min: -5, max: 16 };
  if (lower === "cool") return { min: 5, max: 18 };
  if (lower === "cold" || lower === "winter") return { min: -8, max: 12 };
  if (lower.includes("all-season") || lower.includes("year-round")) return { min: -5, max: 30 };
  return { min: null, max: null };
}

function inferSubcategory(category: string, name: string, fabric: string): string | null {
  const text = `${name} ${fabric}`.toLowerCase();
  const candidates = [
    "boot",
    "loafer",
    "trainer",
    "chino",
    "jean",
    "cord",
    "oxford",
    "rugby",
    "polo",
    "t-shirt",
    "blazer",
    "jacket",
    "coat",
    "sock",
    "belt",
    "pocket square"
  ];
  const found = candidates.find((candidate) => text.includes(candidate));
  return found ?? category;
}

function inferFormality(category: string, name: string): number {
  const text = name.toLowerCase();
  if (category === "sock") return 1;
  if (text.includes("boot") || text.includes("loafer") || text.includes("blazer")) return 4;
  if (text.includes("trainer") || text.includes("t-shirt")) return 2;
  if (category === "outerwear" || category === "shirt" || category === "trouser") return 3;
  return 2;
}

function parseQuantity(notes: string): number {
  const match = notes.trim().match(/^(\d+)x\b/i);
  return match ? Number(match[1]) : 1;
}

function parsePrice(raw: string): number | null {
  const cleaned = raw.replace(/[£,\s]/g, "");
  if (!cleaned) return null;
  const value = Number(cleaned);
  return Number.isFinite(value) ? value : null;
}

function normalizeImportedItem(
  category: string,
  name: string,
  colour: string | null,
  subcategory: string | null
): { name: string; subcategory: string | null } {
  if (category === "footwear" && colour && !name.toLowerCase().includes(colour.toLowerCase())) {
    return { name: `${name} — ${colour.toLowerCase()}`, subcategory };
  }
  return { name, subcategory };
}

const columns = [
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
  "price",
  "currency",
  "acquired_date",
  "link",
  "ref_code",
  "quantity"
];

const statements: string[] = [
  "PRAGMA foreign_keys = OFF;",
  "DELETE FROM wear_log;",
  "DELETE FROM suggestion_items;",
  "DELETE FROM suggestion_sets;",
  "DELETE FROM items;",
  "DELETE FROM meta WHERE key = 'last_import';"
];

for (const [index, row] of dataRows.entries()) {
  const [
    rawCategory,
    name,
    rawColour,
    fabric,
    brand,
    size,
    season,
    rawStatus,
    notes,
    sourceDetail,
    rawPrice,
    acquired,
    link,
    refCode
  ] = row;
  const category = categoryMap[rawCategory] ?? "other";
  const { status, fitNotes } = parseStatus(rawStatus ?? "");
  const { colour, pattern } = parseColourPattern(rawColour ?? "");
  const temp = inferTemp(season ?? "");
  const noteText = notes?.trim() || null;
  const normalized = normalizeImportedItem(
    category,
    name.trim(),
    colour,
    inferSubcategory(category, name, fabric ?? "")
  );
  const values = [
    idFor(row, index),
    normalized.name,
    category,
    normalized.subcategory,
    colour,
    pattern,
    fabric?.trim() || null,
    brand?.trim() || null,
    size?.trim() || null,
    fitNotes,
    JSON.stringify([season?.trim() || "unspecified"]),
    temp.min,
    temp.max,
    inferFormality(category, name),
    status,
    noteText,
    sourceDetail?.trim() || null,
    [name, rawColour, fabric, brand, refCode].filter(Boolean).join(" "),
    parsePrice(rawPrice ?? ""),
    "GBP",
    acquired?.trim() || null,
    link?.trim() || null,
    refCode?.trim() || null,
    parseQuantity(notes ?? "")
  ];
  statements.push(
    `INSERT INTO items (${columns.join(", ")}) VALUES (${values.map(sql).join(", ")});`
  );
}

statements.push("INSERT INTO items_fts(items_fts) VALUES('rebuild');");
statements.push(
  `INSERT INTO meta(key, value) VALUES ('last_import', ${sql(
    JSON.stringify({ source: inputPath, imported_at: new Date().toISOString(), rows: dataRows.length })
  )});`
);
statements.push("PRAGMA foreign_keys = ON;");

const outputPath = resolve("seed/wardrobe-import.sql");
mkdirSync(dirname(outputPath), { recursive: true });
writeFileSync(outputPath, `${statements.join("\n")}\n`);

console.log(`Wrote ${outputPath}`);
console.log(`Rows: ${dataRows.length}`);
