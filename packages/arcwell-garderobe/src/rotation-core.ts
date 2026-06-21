import type { Item } from "./types";
import {
  calculateAvailability,
  findItem,
  itemFields,
  queryAll,
  queryFirst
} from "./db";
import { table, ulidLike } from "./util";

export const rotationRoles = ["outerwear", "shirt", "trouser", "sock", "footwear", "tie"] as const;

export const roleCategories: Record<string, string> = {
  outerwear: "outerwear",
  shirt: "shirt",
  knitwear: "knitwear",
  trouser: "trouser",
  sock: "sock",
  footwear: "footwear",
  tie: "accessory",
  scarf: "accessory",
  belt: "accessory",
  accessory: "accessory"
};

export type Rotation = {
  id: string;
  name: string;
  season: string | null;
  days: number;
  notes: string | null;
  active: number;
  start_date: string | null;
  updated_at: string;
};

export type RotationSlotRow = {
  rotation_id: string;
  day_number: number;
  role: string;
  item_id: string;
  item: string;
  category: string;
  colour: string | null;
  fit_notes: string | null;
  status: string;
  quantity: number;
};

export function isoDate(input?: string): string {
  if (input) return input;
  const parts = new Intl.DateTimeFormat("en-GB", {
    timeZone: "Europe/London",
    year: "numeric",
    month: "2-digit",
    day: "2-digit"
  }).formatToParts(new Date());
  const byType = new Map(parts.map((part) => [part.type, part.value]));
  return `${byType.get("year")}-${byType.get("month")}-${byType.get("day")}`;
}

export function addDays(dateIso: string, days: number): string {
  const date = new Date(`${dateIso}T00:00:00Z`);
  date.setUTCDate(date.getUTCDate() + days);
  return date.toISOString().slice(0, 10);
}

export function computedDay(rotation: Rotation, dateIso = isoDate()): number | null {
  if (!rotation.start_date) return null;
  const start = Date.parse(`${rotation.start_date}T00:00:00Z`);
  const date = Date.parse(`${dateIso}T00:00:00Z`);
  const diff = Math.floor((date - start) / 86400000);
  return ((diff % rotation.days) + rotation.days) % rotation.days + 1;
}

export async function getRotation(db: D1Database, name?: string): Promise<Rotation> {
  const row = name
    ? await queryFirst<Rotation>(db, "SELECT * FROM rotations WHERE name = ?", [name])
    : await queryFirst<Rotation>(db, "SELECT * FROM rotations WHERE active = 1 LIMIT 1");
  if (!row) throw new Error(name ? `No rotation named ${name}` : "No active rotation");
  return row;
}

export async function resolveRotationDay(
  db: D1Database,
  rotation: Rotation,
  day?: number | "today" | "tomorrow",
  dateIso = isoDate()
): Promise<number | undefined> {
  if (day === undefined) return undefined;
  if (typeof day === "number") return day;
  if (!rotation.start_date) {
    throw new Error(`Rotation ${rotation.name} has no start_date; set it before date-based lookups.`);
  }
  return computedDay(rotation, day === "tomorrow" ? addDays(dateIso, 1) : dateIso) ?? undefined;
}

export async function loadRotationSlots(
  db: D1Database,
  rotationId: string,
  day?: number
): Promise<RotationSlotRow[]> {
  return await queryAll<RotationSlotRow>(
    db,
    `SELECT rs.rotation_id, rs.day_number, rs.role, rs.item_id,
            i.name AS item, i.category, i.colour, i.fit_notes, i.status, i.quantity
     FROM rotation_slots rs
     JOIN items i ON i.id = rs.item_id
     WHERE rs.rotation_id = ? ${day ? "AND rs.day_number = ?" : ""}
       AND i.deleted_at IS NULL
     ORDER BY rs.day_number,
       CASE rs.role
         WHEN 'outerwear' THEN 1
         WHEN 'shirt' THEN 2
         WHEN 'trouser' THEN 3
         WHEN 'sock' THEN 4
         WHEN 'footwear' THEN 5
         WHEN 'tie' THEN 6
         ELSE 99
       END`,
    day ? [rotationId, day] : [rotationId]
  );
}

export function rotationHeader(rotation: Rotation, dateIso = isoDate()): string {
  const today = computedDay(rotation, dateIso);
  const todayText = today ? String(today) : "unset start_date";
  return `Rotation: ${rotation.name}; days=${rotation.days}; start_date=${rotation.start_date ?? "unset"}; today_position=${todayText}`;
}

export function compactDayLine(day: number, rows: RotationSlotRow[]): string {
  if (rows.length === 0) return `${day} · — unassigned —`;
  const byRole = new Map(rows.map((row) => [row.role, `${row.item} [${row.colour ?? "n/a"}]`]));
  const parts = rotationRoles.map((role) => byRole.get(role) ?? (role === "tie" ? "(no tie)" : "—"));
  return `${day} · ${parts.join(" / ")}`;
}

export async function formatRotationDay(
  db: D1Database,
  rotation: Rotation,
  day: number,
  dateIso = isoDate()
): Promise<string> {
  const rows = await loadRotationSlots(db, rotation.id, day);
  if (rows.length === 0) return `${rotationHeader(rotation, dateIso)}\n${day} · — unassigned —`;
  const items = await queryAll<Item>(
    db,
    `SELECT ${itemFields.join(", ")} FROM items WHERE id IN (${rows.map(() => "?").join(",")})`,
    rows.map((row) => row.item_id)
  );
  const availability = await calculateAvailability(db, items, dateIso);
  return [
    rotationHeader(rotation, dateIso),
    table(
      rows.map((row) => ({
        day: row.day_number,
        role: row.role,
        id: row.item_id,
        item: row.item,
        colour: row.colour ?? "",
        fit: row.fit_notes ?? "",
        status: row.status,
        p_available: availability.get(row.item_id)?.label ?? "1.00"
      }))
    )
  ].join("\n");
}

export function warningForSlot(day: number, role: string, item: Item): string | null {
  if (["benched", "retired", "secondary"].includes(item.status)) {
    return `day ${day} ${role}: ${item.name} is ${item.status}`;
  }
  return null;
}

export async function resolveItemForSlot(db: D1Database, value: string, role: string): Promise<Item> {
  const item = await findItem(db, value);
  if (!item) throw new Error(`No item found for rotation ${role}: ${value}`);
  const expected = roleCategories[role] ?? role;
  if (item.category !== expected) {
    throw new Error(`Rotation ${role} requires category ${expected}; ${value} resolved to ${item.category}`);
  }
  return item;
}

export async function rotationMarkers(
  db: D1Database,
  dateIso: string,
  horizonDays = 2
): Promise<{ header: string[]; markers: Map<string, string[]> }> {
  const markers = new Map<string, string[]>();
  let rotation: Rotation;
  try {
    rotation = await getRotation(db);
  } catch {
    return { header: ["rotation: none active"], markers };
  }
  if (!rotation.start_date) {
    return { header: [`rotation ${rotation.name}: start_date unset`], markers };
  }
  const header: string[] = [];
  for (let offset = 0; offset < horizonDays; offset++) {
    const date = addDays(dateIso, offset);
    const day = computedDay(rotation, date);
    if (!day) continue;
    const rows = await loadRotationSlots(db, rotation.id, day);
    header.push(`rotation ${offset === 0 ? "today" : "tomorrow"} d${day}: ${compactDayLine(day, rows)}`);
    for (const row of rows) {
      const existing = markers.get(row.item_id) ?? [];
      existing.push(`rot d${day}`);
      markers.set(row.item_id, existing);
    }
  }
  return { header, markers };
}

export async function setRotationDaySlots(
  db: D1Database,
  rotation: Rotation,
  day: number,
  slots: Record<string, string>,
  replace: boolean
): Promise<{ warnings: string[]; text: string }> {
  if (day < 1 || day > rotation.days) {
    throw new Error(`Day ${day} is outside rotation ${rotation.name} range 1-${rotation.days}`);
  }
  const resolved: Array<{ role: string; item: Item }> = [];
  const warnings: string[] = [];
  for (const [role, value] of Object.entries(slots)) {
    const item = await resolveItemForSlot(db, value, role);
    const warning = warningForSlot(day, role, item);
    if (warning) warnings.push(warning);
    resolved.push({ role, item });
  }
  const statements: D1PreparedStatement[] = [];
  if (replace) {
    statements.push(db.prepare("DELETE FROM rotation_slots WHERE rotation_id = ? AND day_number = ?").bind(rotation.id, day));
  } else {
    for (const { role } of resolved) {
      statements.push(
        db.prepare("DELETE FROM rotation_slots WHERE rotation_id = ? AND day_number = ? AND role = ?").bind(
          rotation.id,
          day,
          role
        )
      );
    }
  }
  for (const { role, item } of resolved) {
    statements.push(
      db.prepare("INSERT INTO rotation_slots(id, rotation_id, day_number, role, item_id) VALUES (?, ?, ?, ?, ?)").bind(
        ulidLike("slot_"),
        rotation.id,
        day,
        role,
        item.id
      )
    );
  }
  await db.batch(statements);
  return { warnings, text: await formatRotationDay(db, rotation, day) };
}
