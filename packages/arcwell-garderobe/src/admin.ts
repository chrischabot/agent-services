import { findItem, queryAll, run, searchItems, updateItemByIdOrRef } from "./db";
import type { Env } from "./types";
import { escapeHtml, html, json } from "./util";
import { handleLogin, requireAdmin } from "./auth";

function page(title: string, body: string): Response {
  return html(`<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>${escapeHtml(title)}</title>
  <style>
    body { margin: 0; font: 14px/1.45 system-ui, -apple-system, Segoe UI, sans-serif; background: #f7f7f4; color: #222; }
    header { display: flex; align-items: center; justify-content: space-between; padding: 14px 18px; border-bottom: 1px solid #ddd8ce; background: #fffefa; position: sticky; top: 0; }
    main { padding: 18px; }
    h1 { font-size: 20px; margin: 0; }
    a { color: #17584c; }
    table { width: 100%; border-collapse: collapse; background: white; }
    th, td { border-bottom: 1px solid #ebe7df; padding: 8px; text-align: left; vertical-align: top; }
    th { font-size: 12px; text-transform: uppercase; color: #61584c; background: #fbfaf6; }
    input, select, textarea { width: 100%; min-width: 110px; box-sizing: border-box; font: inherit; padding: 6px; border: 1px solid #c9c1b3; border-radius: 4px; background: white; }
    textarea { min-height: 40px; }
    button { padding: 7px 10px; border: 0; border-radius: 5px; background: #1c5f52; color: white; font-weight: 700; cursor: pointer; }
    .filters { display: flex; gap: 10px; align-items: end; margin-bottom: 16px; flex-wrap: wrap; }
    .filters label { font-size: 12px; font-weight: 700; color: #61584c; }
    .muted { color: #756d63; }
  </style>
</head>
<body>
  <header><h1>Garderobe</h1><nav><a href="/admin">Inventory</a> · <a href="/admin/stats">Stats</a></nav></header>
  <main>${body}</main>
</body>
</html>`);
}

export async function handleAdmin(request: Request, env: Env): Promise<Response> {
  const url = new URL(request.url);

  if (url.pathname === "/login" && request.method === "POST") return await handleLogin(request, env);

  const denied = await requireAdmin(request, env);
  if (denied) return denied;

  if (url.pathname === "/admin/item" && request.method === "GET") {
    const item = await findItem(env.DB, url.searchParams.get("id") ?? "");
    if (!item) return page("Not found", "<p>Item not found.</p>");
    return json(item);
  }

  if (url.pathname === "/admin/items/update" && request.method === "POST") {
    const form = await request.formData();
    const id = String(form.get("id") ?? "");
    await updateItemByIdOrRef(env.DB, id, {
      status: String(form.get("status") ?? "active"),
      temp_min_c: form.get("temp_min_c") ? Number(form.get("temp_min_c")) : null,
      temp_max_c: form.get("temp_max_c") ? Number(form.get("temp_max_c")) : null,
      notes: String(form.get("notes") ?? "")
    });
    return Response.redirect(new URL("/admin", request.url).toString(), 303);
  }

  if (url.pathname === "/admin/stats") {
    const stats = await queryAll<Record<string, unknown>>(
      env.DB,
      `SELECT category, status, count(*) AS items, round(sum(coalesce(price,0)), 2) AS spend_gbp
       FROM items
       WHERE deleted_at IS NULL
       GROUP BY category, status
       ORDER BY category, status`
    );
    const imported = await env.DB.prepare("SELECT value FROM meta WHERE key = 'last_import'").first<{ value: string }>();
    return page(
      "Garderobe Stats",
      `<p class="muted">Last import: ${escapeHtml(imported?.value ?? "unknown")}</p><table><thead><tr><th>Category</th><th>Status</th><th>Items</th><th>Spend GBP</th></tr></thead><tbody>${stats
        .map(
          (row) =>
            `<tr><td>${escapeHtml(row.category)}</td><td>${escapeHtml(row.status)}</td><td>${escapeHtml(row.items)}</td><td>${escapeHtml(row.spend_gbp)}</td></tr>`
        )
        .join("")}</tbody></table>`
    );
  }

  if (url.pathname !== "/admin") return new Response("Not found", { status: 404 });

  const category = url.searchParams.get("category") ?? undefined;
  const status = url.searchParams.get("status") ?? undefined;
  const q = url.searchParams.get("q") ?? undefined;
  const rows = await searchItems(env.DB, { category, status, query: q, limit: 100 });
  const count = await env.DB.prepare("SELECT count(*) AS count FROM items WHERE deleted_at IS NULL").first<{ count: number }>();
  return page(
    "Garderobe Inventory",
    `<form class="filters" method="get">
      <label>Search<br><input name="q" value="${escapeHtml(q ?? "")}"></label>
      <label>Category<br><input name="category" value="${escapeHtml(category ?? "")}"></label>
      <label>Status<br><input name="status" value="${escapeHtml(status ?? "")}"></label>
      <button>Filter</button>
      <span class="muted">${count?.count ?? rows.length} total items</span>
    </form>
    <table>
      <thead><tr><th>Name</th><th>Category</th><th>Brand</th><th>Colour</th><th>Status</th><th>Temp</th><th>Notes</th><th></th></tr></thead>
      <tbody>${rows
        .map(
          (item) => `<tr>
        <form method="post" action="/admin/items/update">
          <input type="hidden" name="id" value="${escapeHtml(item.id)}">
          <td><a href="/admin/item?id=${encodeURIComponent(item.id)}">${escapeHtml(item.name)}</a><br><span class="muted">${escapeHtml(item.ref_code ?? item.id)}</span></td>
          <td>${escapeHtml(item.category)}</td>
          <td>${escapeHtml(item.brand ?? "")}</td>
          <td>${escapeHtml(item.colour ?? "")}</td>
          <td><select name="status">${["active", "breaking_in", "benched", "secondary", "retired"]
            .map((s) => `<option value="${s}" ${s === item.status ? "selected" : ""}>${s}</option>`)
            .join("")}</select></td>
          <td><input name="temp_min_c" value="${escapeHtml(item.temp_min_c ?? "")}"><input name="temp_max_c" value="${escapeHtml(item.temp_max_c ?? "")}"></td>
          <td><textarea name="notes">${escapeHtml(item.notes ?? "")}</textarea></td>
          <td><button>Save</button></td>
        </form>
      </tr>`
        )
        .join("")}</tbody>
    </table>`
  );
}

export async function exportInventoryCsv(env: Env): Promise<string> {
  const rows = await queryAll<Record<string, unknown>>(
    env.DB,
    "SELECT * FROM items WHERE deleted_at IS NULL ORDER BY category, name"
  );
  const columns = rows[0] ? Object.keys(rows[0]) : [];
  const quote = (value: unknown) => `"${String(value ?? "").replaceAll('"', '""')}"`;
  return [columns.map(quote).join(","), ...rows.map((row) => columns.map((col) => quote(row[col])).join(","))].join("\n");
}
