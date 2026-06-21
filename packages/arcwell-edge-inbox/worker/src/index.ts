export interface Env {
  ARCWELL_EDGE_SECRET: string;
  ARCWELL_EDGE_NEXT_SECRET?: string;
  TELEGRAM_WEBHOOK_SECRET?: string;
  MAX_PAYLOAD_BYTES?: string;
  RATE_LIMIT_WINDOW_SECONDS?: string;
  RATE_LIMIT_MAX_EVENTS?: string;
  EDGE_DB?: D1Database;
}

type EdgeEventInput = {
  source?: unknown;
  idempotencyKey?: unknown;
  payload?: unknown;
  maxAgeSeconds?: unknown;
};

type LeaseInput = {
  leaseSeconds?: unknown;
};

type AckInput = {
  idempotencyKey?: unknown;
};

type NackInput = {
  idempotencyKey?: unknown;
  error?: unknown;
  retrySeconds?: unknown;
};

export type StoredEdgeEvent = {
  source: string;
  idempotencyKey: string;
  payload: unknown;
  status: "pending" | "leased" | "acked" | "failed" | "dead_lettered" | "expired";
  receivedAt: number;
  expiresAt: number;
  leasedUntil: number | null;
  attempts: number;
  maxAttempts: number;
  error: string | null;
};

export interface EdgeEventStore {
  enqueue(event: StoredEdgeEvent): Promise<{ event: StoredEdgeEvent; duplicate: boolean }>;
  checkRateLimit(key: string, now: number, windowSeconds: number, maxEvents: number): Promise<RateLimitResult>;
  lease(now: number, leaseSeconds: number): Promise<StoredEdgeEvent | null>;
  ack(idempotencyKey: string): Promise<StoredEdgeEvent | null>;
  nack(idempotencyKey: string, error: string, retrySeconds: number, now: number): Promise<StoredEdgeEvent | null>;
  list(now: number, limit: number): Promise<StoredEdgeEvent[]>;
}

type RateLimitResult = {
  allowed: boolean;
  limit: number;
  remaining: number;
  resetAt: number;
};

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    if (!env.EDGE_DB) {
      const url = new URL(request.url);
      if (request.method === "GET" && url.pathname === "/health") {
        return json({ ok: false, service: "arcwell-edge-inbox", error: "missing EDGE_DB binding" }, 503);
      }
      return json({ error: "missing_edge_db_binding" }, 503);
    }
    return handleRequest(request, env, new D1EdgeEventStore(env.EDGE_DB));
  }
};

export async function handleRequest(request: Request, env: Env, store: EdgeEventStore): Promise<Response> {
  const url = new URL(request.url);
  if (request.method === "GET" && url.pathname === "/health") {
    return json({ ok: true, service: "arcwell-edge-inbox", durable: true });
  }
  if (request.method === "POST" && url.pathname === "/telegram/webhook") {
    if (!authorized(request, env) && !authorizedTelegram(request, env)) {
      return json({ error: "unauthorized" }, 401);
    }
    return enqueueTelegramUpdate(request, env, store);
  }
  if (!authorized(request, env)) {
    return json({ error: "unauthorized" }, 401);
  }
  if (request.method === "POST" && url.pathname === "/events") {
    return enqueueEvent(request, env, store);
  }
  if (request.method === "POST" && url.pathname === "/drain/lease") {
    const input = await readJson<LeaseInput>(request, env);
    if ("response" in input) return input.response;
    const leaseSeconds = clampNumber(input.value.leaseSeconds, 1, 900, 120);
    const event = await store.lease(Date.now(), leaseSeconds);
    return json({ event });
  }
  if (request.method === "POST" && url.pathname === "/drain/ack") {
    const input = await readJson<AckInput>(request, env);
    if ("response" in input) return input.response;
    const idempotencyKey = validString(input.value.idempotencyKey, "idempotency_key", 200);
    if ("response" in idempotencyKey) return idempotencyKey.response;
    const event = await store.ack(idempotencyKey.value);
    return json({ ok: event !== null, event });
  }
  if (request.method === "POST" && url.pathname === "/drain/nack") {
    const input = await readJson<NackInput>(request, env);
    if ("response" in input) return input.response;
    const idempotencyKey = validString(input.value.idempotencyKey, "idempotency_key", 200);
    if ("response" in idempotencyKey) return idempotencyKey.response;
    const error = validString(input.value.error, "error", 2000);
    if ("response" in error) return error.response;
    const retrySeconds = clampNumber(input.value.retrySeconds, 1, 3600, 60);
    const event = await store.nack(idempotencyKey.value, error.value, retrySeconds, Date.now());
    return json({ ok: event !== null, event });
  }
  if (request.method === "GET" && url.pathname === "/events") {
    const limit = clampNumber(url.searchParams.get("limit"), 1, 100, 25);
    return json({ events: await store.list(Date.now(), limit) });
  }
  return json({ error: "not_found" }, 404);
}

async function enqueueEvent(request: Request, env: Env, store: EdgeEventStore): Promise<Response> {
  const input = await readJson<EdgeEventInput>(request, env);
  if ("response" in input) return input.response;
  const source = validString(input.value.source, "source", 200);
  if ("response" in source) return source.response;
  const idempotencyKey = validString(input.value.idempotencyKey, "idempotency_key", 200);
  if ("response" in idempotencyKey) return idempotencyKey.response;
  const maxAgeSeconds = clampNumber(input.value.maxAgeSeconds, 60, 86400, 3600);
  const now = Date.now();
  const limited = await enforceRateLimit(store, env, `source:${source.value}`, now);
  if (limited) return limited;
  const { event, duplicate } = await store.enqueue({
    source: source.value,
    idempotencyKey: idempotencyKey.value,
    payload: input.value.payload ?? null,
    status: "pending",
    receivedAt: now,
    expiresAt: now + maxAgeSeconds * 1000,
    leasedUntil: null,
    attempts: 0,
    maxAttempts: 3,
    error: null
  });
  return json({
    accepted: true,
    duplicate,
    source: event.source,
    idempotencyKey: event.idempotencyKey,
    status: event.status,
    expiresAt: event.expiresAt
  });
}

async function enqueueTelegramUpdate(request: Request, env: Env, store: EdgeEventStore): Promise<Response> {
  const input = await readJson<Record<string, unknown>>(request, env);
  if ("response" in input) return input.response;
  const update = normalizeTelegramUpdate(input.value);
  if ("response" in update) return update.response;
  const now = Date.now();
  const limited = await enforceRateLimit(store, env, `source:telegram:chat:${update.value.chatId}`, now);
  if (limited) return limited;
  const { event, duplicate } = await store.enqueue({
    source: "telegram",
    idempotencyKey: `telegram:update:${update.value.updateId}`,
    payload: update.value,
    status: "pending",
    receivedAt: now,
    expiresAt: now + 24 * 60 * 60 * 1000,
    leasedUntil: null,
    attempts: 0,
    maxAttempts: 3,
    error: null
  });
  return json({
    accepted: true,
    duplicate,
    source: event.source,
    idempotencyKey: event.idempotencyKey,
    status: event.status
  });
}

function normalizeTelegramUpdate(value: Record<string, unknown>): { value: Record<string, unknown> } | { response: Response } {
  const updateId = value.update_id;
  if (typeof updateId !== "number" || !Number.isInteger(updateId)) {
    return { response: json({ error: "invalid_update_id" }, 400) };
  }
  const message = objectValue(value.message) ?? objectValue(value.edited_message);
  if (!message) {
    return { response: json({ error: "unsupported_telegram_update" }, 400) };
  }
  const chat = objectValue(message.chat);
  const from = objectValue(message.from);
  const text = typeof message.text === "string" ? message.text : typeof message.caption === "string" ? message.caption : null;
  if (!chat || text === null) {
    return { response: json({ error: "unsupported_telegram_message" }, 400) };
  }
  const chatId = chat.id;
  if (typeof chatId !== "number" && typeof chatId !== "string") {
    return { response: json({ error: "invalid_chat_id" }, 400) };
  }
  const messageId = message.message_id;
  if (typeof messageId !== "number" && typeof messageId !== "string") {
    return { response: json({ error: "invalid_message_id" }, 400) };
  }
  return {
    value: {
      updateId,
      chatId,
      messageId,
      senderId: from?.id ?? null,
      username: typeof from?.username === "string" ? from.username : null,
      date: message.date ?? null,
      text
    }
  };
}

function objectValue(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null && !Array.isArray(value) ? (value as Record<string, unknown>) : null;
}

function authorized(request: Request, env: Env): boolean {
  const provided = request.headers.get("x-arcwell-edge-secret");
  if (!provided) return false;
  return [env.ARCWELL_EDGE_SECRET, env.ARCWELL_EDGE_NEXT_SECRET]
    .filter((secret): secret is string => typeof secret === "string" && secret.length > 0)
    .some((secret) => provided === secret);
}

function authorizedTelegram(request: Request, env: Env): boolean {
  const configured = env.TELEGRAM_WEBHOOK_SECRET;
  if (typeof configured !== "string" || configured.length === 0) return false;
  return request.headers.get("x-telegram-bot-api-secret-token") === configured;
}

async function enforceRateLimit(store: EdgeEventStore, env: Env, key: string, now: number): Promise<Response | null> {
  const windowSeconds = clampNumber(env.RATE_LIMIT_WINDOW_SECONDS, 1, 3600, 60);
  const maxEvents = clampNumber(env.RATE_LIMIT_MAX_EVENTS, 1, 10000, 120);
  const result = await store.checkRateLimit(key, now, windowSeconds, maxEvents);
  if (result.allowed) return null;
  return json(
    {
      error: "rate_limited",
      limit: result.limit,
      remaining: result.remaining,
      resetAt: result.resetAt
    },
    429,
    { "retry-after": String(Math.max(1, Math.ceil((result.resetAt - now) / 1000))) }
  );
}

async function readJson<T>(request: Request, env: Env): Promise<{ value: T } | { response: Response }> {
  const raw = await request.text();
  const maxBytes = Number(env.MAX_PAYLOAD_BYTES ?? "64000");
  if (new TextEncoder().encode(raw).byteLength > maxBytes) {
    return { response: json({ error: "payload_too_large" }, 413) };
  }
  try {
    return { value: JSON.parse(raw) as T };
  } catch {
    return { response: json({ error: "invalid_json" }, 400) };
  }
}

function validString(value: unknown, label: string, maxLength: number): { value: string } | { response: Response } {
  if (typeof value !== "string" || value.length === 0 || value.length > maxLength) {
    return { response: json({ error: `invalid_${label}` }, 400) };
  }
  return { value };
}

function clampNumber(value: unknown, min: number, max: number, fallback: number): number {
  const parsed =
    typeof value === "number" ? value : typeof value === "string" && value.length > 0 ? Number(value) : fallback;
  if (!Number.isFinite(parsed)) return fallback;
  return Math.max(min, Math.min(max, Math.trunc(parsed)));
}

function json(value: unknown, status = 200, headers: Record<string, string> = {}): Response {
  return new Response(JSON.stringify(value), {
    status,
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": "no-store",
      ...headers
    }
  });
}

export class MemoryEdgeEventStore implements EdgeEventStore {
  private events = new Map<string, StoredEdgeEvent>();
  private rateLimits = new Map<string, { windowStart: number; count: number }>();

  async enqueue(event: StoredEdgeEvent): Promise<{ event: StoredEdgeEvent; duplicate: boolean }> {
    const existing = this.events.get(event.idempotencyKey);
    if (existing) return { event: existing, duplicate: true };
    this.events.set(event.idempotencyKey, { ...event });
    return { event, duplicate: false };
  }

  async checkRateLimit(key: string, now: number, windowSeconds: number, maxEvents: number): Promise<RateLimitResult> {
    const windowMs = windowSeconds * 1000;
    const existing = this.rateLimits.get(key);
    const current =
      existing && existing.windowStart + windowMs > now ? existing : { windowStart: now, count: 0 };
    current.count += 1;
    this.rateLimits.set(key, current);
    const remaining = Math.max(0, maxEvents - current.count);
    return {
      allowed: current.count <= maxEvents,
      limit: maxEvents,
      remaining,
      resetAt: current.windowStart + windowMs
    };
  }

  async lease(now: number, leaseSeconds: number): Promise<StoredEdgeEvent | null> {
    this.expire(now);
    const candidates = [...this.events.values()]
      .filter(
        (event) =>
          (event.status === "pending" && event.attempts < event.maxAttempts) ||
          (event.status === "failed" &&
            event.attempts < event.maxAttempts &&
            (event.leasedUntil === null || event.leasedUntil <= now)) ||
          (event.status === "leased" && event.leasedUntil !== null && event.leasedUntil <= now)
      )
      .sort((a, b) => a.receivedAt - b.receivedAt);
    const event = candidates[0];
    if (!event) return null;
    event.status = "leased";
    event.attempts += 1;
    event.leasedUntil = now + leaseSeconds * 1000;
    event.error = null;
    return { ...event };
  }

  async ack(idempotencyKey: string): Promise<StoredEdgeEvent | null> {
    const event = this.events.get(idempotencyKey);
    if (!event) return null;
    event.status = "acked";
    event.leasedUntil = null;
    return { ...event };
  }

  async nack(idempotencyKey: string, error: string, retrySeconds: number, now: number): Promise<StoredEdgeEvent | null> {
    const event = this.events.get(idempotencyKey);
    if (!event) return null;
    event.error = error;
    event.leasedUntil = now + retrySeconds * 1000;
    event.status = event.attempts >= event.maxAttempts ? "dead_lettered" : "failed";
    return { ...event };
  }

  async list(now: number, limit: number): Promise<StoredEdgeEvent[]> {
    this.expire(now);
    return [...this.events.values()]
      .sort((a, b) => a.receivedAt - b.receivedAt)
      .slice(0, limit)
      .map((event) => ({ ...event }));
  }

  private expire(now: number): void {
    for (const event of this.events.values()) {
      if (event.expiresAt <= now && event.status !== "acked" && event.status !== "dead_lettered") {
        event.status = "expired";
        event.leasedUntil = null;
      }
    }
  }
}

export class D1EdgeEventStore implements EdgeEventStore {
  constructor(private readonly db: D1Database) {}

  async enqueue(event: StoredEdgeEvent): Promise<{ event: StoredEdgeEvent; duplicate: boolean }> {
    await this.ensureSchema();
    const existing = await this.get(event.idempotencyKey);
    if (existing) return { event: existing, duplicate: true };
    await this.db
      .prepare(
        `INSERT INTO edge_events
          (source, idempotency_key, payload_json, status, received_at, expires_at, leased_until, attempts, max_attempts, error)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, NULL)`
      )
      .bind(
        event.source,
        event.idempotencyKey,
        JSON.stringify(event.payload),
        event.status,
        event.receivedAt,
        event.expiresAt,
        event.attempts,
        event.maxAttempts
      )
      .run();
    return { event, duplicate: false };
  }

  async checkRateLimit(key: string, now: number, windowSeconds: number, maxEvents: number): Promise<RateLimitResult> {
    await this.ensureSchema();
    const windowMs = windowSeconds * 1000;
    const existing = await this.db
      .prepare("SELECT key, window_start, count FROM edge_rate_limits WHERE key = ?1")
      .bind(key)
      .first<RateLimitRow>();
    const windowStart = existing && existing.window_start + windowMs > now ? existing.window_start : now;
    const count = existing && existing.window_start === windowStart ? existing.count + 1 : 1;
    await this.db
      .prepare(
        `INSERT INTO edge_rate_limits (key, window_start, count)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET window_start = excluded.window_start, count = excluded.count`
      )
      .bind(key, windowStart, count)
      .run();
    return {
      allowed: count <= maxEvents,
      limit: maxEvents,
      remaining: Math.max(0, maxEvents - count),
      resetAt: windowStart + windowMs
    };
  }

  async lease(now: number, leaseSeconds: number): Promise<StoredEdgeEvent | null> {
    await this.ensureSchema();
    await this.expire(now);
    const row = await this.db
      .prepare(
        `SELECT * FROM edge_events
         WHERE attempts < max_attempts
           AND (
             status = 'pending'
             OR (status = 'failed' AND (leased_until IS NULL OR leased_until <= ?1))
             OR (status = 'leased' AND leased_until IS NOT NULL AND leased_until <= ?1)
           )
         ORDER BY received_at ASC
         LIMIT 1`
      )
      .bind(now)
      .first<EdgeEventRow>();
    if (!row || row.attempts >= row.max_attempts) return null;
    const leasedUntil = now + leaseSeconds * 1000;
    await this.db
      .prepare(
        `UPDATE edge_events
         SET status = 'leased', attempts = attempts + 1, leased_until = ?2, error = NULL
         WHERE idempotency_key = ?1`
      )
      .bind(row.idempotency_key, leasedUntil)
      .run();
    return this.get(row.idempotency_key);
  }

  async ack(idempotencyKey: string): Promise<StoredEdgeEvent | null> {
    await this.ensureSchema();
    await this.db
      .prepare("UPDATE edge_events SET status = 'acked', leased_until = NULL WHERE idempotency_key = ?1")
      .bind(idempotencyKey)
      .run();
    return this.get(idempotencyKey);
  }

  async nack(idempotencyKey: string, error: string, retrySeconds: number, now: number): Promise<StoredEdgeEvent | null> {
    await this.ensureSchema();
    const event = await this.get(idempotencyKey);
    if (!event) return null;
    const status = event.attempts >= event.maxAttempts ? "dead_lettered" : "failed";
    const leasedUntil = status === "failed" ? now + retrySeconds * 1000 : null;
    await this.db
      .prepare("UPDATE edge_events SET status = ?2, leased_until = ?3, error = ?4 WHERE idempotency_key = ?1")
      .bind(idempotencyKey, status, leasedUntil, error.slice(0, 2000))
      .run();
    return this.get(idempotencyKey);
  }

  async list(now: number, limit: number): Promise<StoredEdgeEvent[]> {
    await this.ensureSchema();
    await this.expire(now);
    const result = await this.db.prepare("SELECT * FROM edge_events ORDER BY received_at ASC LIMIT ?1").bind(limit).all<EdgeEventRow>();
    return result.results.map(eventFromRow);
  }

  private async get(idempotencyKey: string): Promise<StoredEdgeEvent | null> {
    const row = await this.db
      .prepare("SELECT * FROM edge_events WHERE idempotency_key = ?1")
      .bind(idempotencyKey)
      .first<EdgeEventRow>();
    return row ? eventFromRow(row) : null;
  }

  private async expire(now: number): Promise<void> {
    await this.db
      .prepare(
        `UPDATE edge_events
         SET status = 'expired', leased_until = NULL
         WHERE expires_at <= ?1 AND status NOT IN ('acked', 'dead_lettered', 'expired')`
      )
      .bind(now)
      .run();
  }

  private async ensureSchema(): Promise<void> {
    await this.db
      .prepare(
        `CREATE TABLE IF NOT EXISTS edge_events (
          source TEXT NOT NULL,
          idempotency_key TEXT PRIMARY KEY,
          payload_json TEXT NOT NULL,
          status TEXT NOT NULL,
          received_at INTEGER NOT NULL,
          expires_at INTEGER NOT NULL,
          leased_until INTEGER,
          attempts INTEGER NOT NULL DEFAULT 0,
          max_attempts INTEGER NOT NULL DEFAULT 3,
          error TEXT
        )`
      )
      .run();
    await this.db
      .prepare(
        `CREATE TABLE IF NOT EXISTS edge_rate_limits (
          key TEXT PRIMARY KEY,
          window_start INTEGER NOT NULL,
          count INTEGER NOT NULL
        )`
      )
      .run();
  }
}

type RateLimitRow = {
  key: string;
  window_start: number;
  count: number;
};

type EdgeEventRow = {
  source: string;
  idempotency_key: string;
  payload_json: string;
  status: StoredEdgeEvent["status"];
  received_at: number;
  expires_at: number;
  leased_until: number | null;
  attempts: number;
  max_attempts: number;
  error: string | null;
};

function eventFromRow(row: EdgeEventRow): StoredEdgeEvent {
  return {
    source: row.source,
    idempotencyKey: row.idempotency_key,
    payload: JSON.parse(row.payload_json) as unknown,
    status: row.status,
    receivedAt: row.received_at,
    expiresAt: row.expires_at,
    leasedUntil: row.leased_until,
    attempts: row.attempts,
    maxAttempts: row.max_attempts,
    error: row.error
  };
}
