export function ulidLike(prefix = ""): string {
  const bytes = new Uint8Array(12);
  crypto.getRandomValues(bytes);
  const body = Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
  return `${prefix}${Date.now().toString(36)}${body}`;
}

export function json(data: unknown, init: ResponseInit = {}): Response {
  return new Response(JSON.stringify(data, null, 2), {
    ...init,
    headers: {
      "content-type": "application/json; charset=utf-8",
      ...init.headers
    }
  });
}

export function html(body: string, init: ResponseInit = {}): Response {
  return new Response(body, {
    ...init,
    headers: {
      "content-type": "text/html; charset=utf-8",
      "x-content-type-options": "nosniff",
      "x-frame-options": "DENY",
      "referrer-policy": "no-referrer",
      "content-security-policy": [
        "default-src 'self'",
        "style-src 'self' 'unsafe-inline'",
        "img-src 'self' https:",
        "form-action 'self'",
        "frame-ancestors 'none'",
        "base-uri 'self'"
      ].join("; "),
      ...init.headers
    }
  });
}

export function escapeHtml(value: unknown): string {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

export function compact<T>(values: Array<T | null | undefined | false | "">): T[] {
  return values.filter(Boolean) as T[];
}

export function table(rows: Array<Record<string, unknown>>, columns?: string[]): string {
  if (rows.length === 0) return "No rows.";
  const cols = columns ?? Object.keys(rows[0]);
  const widths = cols.map((col) =>
    Math.max(col.length, ...rows.map((row) => String(row[col] ?? "").length))
  );
  const line = cols.map((col, i) => col.padEnd(widths[i])).join(" | ");
  const sep = widths.map((w) => "-".repeat(w)).join("-|-");
  const body = rows
    .map((row) => cols.map((col, i) => String(row[col] ?? "").padEnd(widths[i])).join(" | "))
    .join("\n");
  return `${line}\n${sep}\n${body}`;
}
