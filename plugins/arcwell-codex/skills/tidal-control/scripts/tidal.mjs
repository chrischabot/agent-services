#!/usr/bin/env node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import crypto from "node:crypto";

const DEFAULT_LIMIT = 100;
const BAD_MATCH = /\b(live|karaoke|tribute|cover|instrumental|demo|rehearsal|unplugged|acoustic|remix)\b/i;

function usage(exitCode = 0) {
  const text = `
Usage:
  tidal.mjs session [--json]
  tidal.mjs playlists [--filter TEXT] [--limit N] [--json]
  tidal.mjs playlist --playlist NAME_OR_ID [--json]
  tidal.mjs search-tracks --query TEXT [--limit N] [--json]
  tidal.mjs create-playlist --name NAME [--description TEXT] [--public|--unlisted] [--reuse-existing] [--dry-run] [--json]
  tidal.mjs update-playlist --playlist NAME_OR_ID [--name NAME] [--description TEXT] [--public|--unlisted] [--dry-run] [--json]
  tidal.mjs add-tracks --playlist NAME_OR_ID [--track "Artist - Title"]... [--tracks-file FILE] [--allow-duplicates] [--dry-run] [--json]
  tidal.mjs favorite-tracks [--track "Artist - Title"]... [--tracks-file FILE] [--dry-run] [--json]
  tidal.mjs favorite-playlist --playlist NAME_OR_ID [--dry-run] [--json]
`;
  console.log(text.trim());
  process.exit(exitCode);
}

function parseArgs(argv) {
  const command = argv[2];
  const opts = { _: [] };
  for (let i = 3; i < argv.length; i += 1) {
    const arg = argv[i];
    if (!arg.startsWith("--")) {
      opts._.push(arg);
      continue;
    }
    const key = arg.slice(2);
    if (["json", "public", "unlisted", "reuse-existing", "dry-run", "allow-duplicates"].includes(key)) {
      opts[key] = true;
      continue;
    }
    const value = argv[i + 1];
    if (!value || value.startsWith("--")) throw new Error(`missing value for --${key}`);
    i += 1;
    if (key === "track") {
      opts.track = opts.track || [];
      opts.track.push(value);
    } else {
      opts[key] = value;
    }
  }
  return { command, opts };
}

function requireOption(opts, name) {
  if (!opts[name]) throw new Error(`missing required --${name}`);
  return opts[name];
}

function readTextIfExists(file) {
  try {
    return fs.readFileSync(file, "latin1");
  } catch {
    return "";
  }
}

function decodeJwtPayload(token) {
  const payload = token.split(".")[1];
  if (!payload) return {};
  try {
    return JSON.parse(Buffer.from(payload, "base64url").toString("utf8"));
  } catch {
    return {};
  }
}

function extractDesktopSession() {
  const dbDir = path.join(
    os.homedir(),
    "Library/Application Support/TIDAL/IndexedDB/https_desktop.tidal.com_0.indexeddb.leveldb",
  );
  if (!fs.existsSync(dbDir)) return null;
  const files = fs
    .readdirSync(dbDir)
    .filter((name) => /\.(ldb|log)$/i.test(name))
    .map((name) => path.join(dbDir, name));
  let accessToken = null;
  let clientId = null;
  let refreshTokenPresent = false;
  for (const file of files) {
    const text = readTextIfExists(file);
    accessToken ||= text.match(/"accessToken":"([^"]+)"/)?.[1] || null;
    clientId ||= text.match(/"clientId":"([^"]+)"/)?.[1] || null;
    refreshTokenPresent ||= /"refreshToken":"[^"]+"/.test(text);
  }
  if (!accessToken || !clientId) return null;
  return { accessToken, clientId, source: "tidal-desktop-indexeddb", refreshTokenPresent };
}

function getSession() {
  const envToken = process.env.TIDAL_ACCESS_TOKEN;
  const envClient = process.env.TIDAL_CLIENT_ID;
  const session = envToken && envClient
    ? { accessToken: envToken, clientId: envClient, source: "environment", refreshTokenPresent: false }
    : extractDesktopSession();
  if (!session) {
    throw new Error("no TIDAL session found; open/log into the TIDAL desktop app or set TIDAL_ACCESS_TOKEN and TIDAL_CLIENT_ID");
  }
  const payload = decodeJwtPayload(session.accessToken);
  const now = Math.floor(Date.now() / 1000);
  if (payload.exp && payload.exp <= now) {
    throw new Error("TIDAL access token is expired; open the TIDAL desktop app so it can refresh the session");
  }
  return {
    ...session,
    userId: String(payload.uid || process.env.TIDAL_USER_ID || ""),
    countryCode: String(payload.cc || process.env.TIDAL_COUNTRY_CODE || "GB"),
    expiresAt: payload.exp ? new Date(payload.exp * 1000).toISOString() : null,
    scopes: payload.scope || null,
  };
}

async function request(session, url, { method = "GET", body = null, accept = "application/json" } = {}) {
  const headers = {
    Authorization: `Bearer ${session.accessToken}`,
    "x-tidal-token": session.clientId,
    Accept: accept,
  };
  if (body) headers["Content-Type"] = "application/vnd.api+json";
  const res = await fetch(url, {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  let json;
  try {
    json = text ? JSON.parse(text) : {};
  } catch {
    json = { raw: text };
  }
  if (!res.ok) {
    const safe = JSON.stringify(json).replace(/[A-Za-z0-9_-]{80,}/g, "[redacted]");
    throw new Error(`${method} ${url} failed with ${res.status}: ${safe.slice(0, 500)}`);
  }
  return json;
}

function output(data, opts) {
  if (opts.json) {
    console.log(JSON.stringify(data, null, 2));
    return;
  }
  if (Array.isArray(data)) {
    for (const row of data) console.log(formatRow(row));
    return;
  }
  console.log(formatRow(data));
}

function formatRow(row) {
  if (row == null) return "";
  if (typeof row !== "object") return String(row);
  if (row.uuid && row.title) {
    const count = row.numberOfTracks ?? row.numberOfItems ?? "?";
    return `${row.uuid}\t${row.title}\t${count} tracks`;
  }
  if (row.id && row.title && row.artist) return `${row.id}\t${row.artist} - ${row.title}\t[${row.album || ""}]`;
  return JSON.stringify(row);
}

function normalize(value) {
  return String(value || "")
    .toLowerCase()
    .replace(/[’']/g, "'")
    .replace(/^the\s+/, "")
    .replace(/\([^)]*\)/g, "")
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

function compact(value) {
  return normalize(value).replace(/\s+/g, "");
}

function parseTrackLine(line) {
  const cleaned = line.trim();
  if (!cleaned) return null;
  const match = cleaned.match(/^\s*(.+?)\s+-\s+(.+?)\s*$/);
  if (match) return { artist: match[1].trim(), title: match[2].trim(), query: cleaned };
  return { artist: "", title: cleaned, query: cleaned };
}

function loadTrackInputs(opts) {
  const lines = [];
  for (const track of opts.track || []) lines.push(track);
  if (opts["tracks-file"]) {
    const fileText = fs.readFileSync(opts["tracks-file"], "utf8");
    lines.push(...fileText.split(/\r?\n/));
  }
  const parsed = lines.map(parseTrackLine).filter(Boolean);
  if (!parsed.length) throw new Error("provide --track or --tracks-file");
  return parsed;
}

function scoreTrack(item, wanted) {
  const artistText = (item.artists || []).map((artist) => artist.name).join(" ");
  const itemArtist = normalize(artistText);
  const wantArtist = normalize(wanted.artist);
  const itemTitle = normalize(item.title);
  const wantTitle = normalize(wanted.title || wanted.query);
  const compactItemTitle = compact(item.title);
  const compactWantTitle = compact(wanted.title || wanted.query);
  const album = item.album?.title || "";
  let score = 0;
  if (wantArtist && itemArtist === wantArtist) score += 70;
  else if (wantArtist && (itemArtist.includes(wantArtist) || wantArtist.includes(itemArtist))) score += 45;
  if (itemTitle === wantTitle) score += 100;
  else if (compactItemTitle && compactItemTitle === compactWantTitle) score += 96;
  else if (itemTitle.includes(wantTitle) || wantTitle.includes(itemTitle)) score += 60;
  if (BAD_MATCH.test(item.title) || BAD_MATCH.test(album)) score -= 55;
  if (/remaster/i.test(item.title) || /remaster/i.test(album)) score += 3;
  return score;
}

function simplifyTrack(item, score = null) {
  return {
    id: String(item.id),
    title: item.title,
    artist: (item.artists || []).map((artist) => artist.name).join(", "),
    album: item.album?.title || null,
    score,
  };
}

async function searchTracks(session, query, limit = 10) {
  const url = new URL("https://api.tidal.com/v1/search/tracks");
  url.searchParams.set("query", query);
  url.searchParams.set("countryCode", session.countryCode);
  url.searchParams.set("limit", String(limit));
  const json = await request(session, url.toString());
  return json.items || [];
}

async function resolveTrack(session, input) {
  const items = await searchTracks(session, input.query, 12);
  const ranked = items
    .map((item) => ({ item, score: scoreTrack(item, input) }))
    .sort((a, b) => b.score - a.score);
  const best = ranked[0];
  if (!best || best.score < 90) {
    return { ok: false, input: input.query, candidates: ranked.slice(0, 5).map(({ item, score }) => simplifyTrack(item, score)) };
  }
  return { ok: true, input: input.query, track: simplifyTrack(best.item, best.score) };
}

async function listPlaylists(session, opts = {}) {
  if (!session.userId) throw new Error("session token does not include a user id");
  const url = new URL(`https://api.tidal.com/v1/users/${session.userId}/playlists`);
  url.searchParams.set("countryCode", session.countryCode);
  url.searchParams.set("limit", String(opts.limit || DEFAULT_LIMIT));
  const json = await request(session, url.toString());
  let playlists = json.items || [];
  if (opts.filter) {
    const needle = normalize(opts.filter);
    playlists = playlists.filter((playlist) => normalize(playlist.title || playlist.name).includes(needle));
  }
  return playlists.map((playlist) => ({
    uuid: playlist.uuid,
    title: playlist.title || playlist.name,
    description: playlist.description || "",
    numberOfTracks: playlist.numberOfTracks ?? playlist.numberOfItems ?? null,
    publicPlaylist: playlist.publicPlaylist,
  }));
}

async function findPlaylist(session, selector) {
  if (!selector) throw new Error("missing playlist selector");
  if (/^[0-9a-f]{8}-[0-9a-f-]{27,}$/i.test(selector)) return { uuid: selector, title: selector };
  const matches = await listPlaylists(session, { filter: selector, limit: 200 });
  const exact = matches.filter((playlist) => normalize(playlist.title) === normalize(selector));
  if (exact.length === 1) return exact[0];
  if (matches.length === 1) return matches[0];
  if (!matches.length) throw new Error(`no playlist found matching "${selector}"`);
  throw new Error(`multiple playlists match "${selector}": ${matches.map((playlist) => `${playlist.title} (${playlist.uuid})`).join(", ")}`);
}

async function getPlaylistTracks(session, playlistId, limit = 100) {
  const url = new URL(`https://api.tidal.com/v1/playlists/${playlistId}/tracks`);
  url.searchParams.set("countryCode", session.countryCode);
  url.searchParams.set("limit", String(limit));
  const json = await request(session, url.toString());
  return (json.items || []).map((item) => simplifyTrack(item));
}

async function createPlaylist(session, opts) {
  const name = requireOption(opts, "name");
  if (opts["reuse-existing"]) {
    const existing = (await listPlaylists(session, { filter: name, limit: 200 })).find((playlist) => normalize(playlist.title) === normalize(name));
    if (existing) return { reused: true, playlist: existing };
  }
  const body = {
    data: {
      type: "playlists",
      attributes: {
        name,
        accessType: opts.public ? "PUBLIC" : "UNLISTED",
      },
    },
  };
  if (opts.description) body.data.attributes.description = opts.description;
  if (opts["dry-run"]) return { dryRun: true, body };
  const json = await request(session, "https://openapi.tidal.com/v2/playlists", {
    method: "POST",
    body,
    accept: "application/vnd.api+json",
  });
  return {
    created: true,
    playlist: {
      uuid: json.data?.id,
      title: json.data?.attributes?.name,
      numberOfItems: json.data?.attributes?.numberOfItems ?? 0,
    },
  };
}

async function updatePlaylist(session, opts) {
  const playlist = await findPlaylist(session, requireOption(opts, "playlist"));
  const attributes = {};
  if (opts.name) attributes.name = opts.name;
  if (opts.description) attributes.description = opts.description;
  if (opts.public) attributes.accessType = "PUBLIC";
  if (opts.unlisted) attributes.accessType = "UNLISTED";
  if (!Object.keys(attributes).length) throw new Error("provide --name, --description, --public, or --unlisted");
  const body = { data: { id: playlist.uuid, type: "playlists", attributes } };
  if (opts["dry-run"]) return { dryRun: true, playlist, body };
  const json = await request(session, `https://openapi.tidal.com/v2/playlists/${playlist.uuid}`, {
    method: "PATCH",
    body,
    accept: "application/vnd.api+json",
  });
  return { updated: true, playlist: { uuid: json.data?.id, title: json.data?.attributes?.name } };
}

async function addRelationship(session, url, data, opts) {
  if (opts["dry-run"]) return { dryRun: true, url, data };
  const chunks = [];
  for (let i = 0; i < data.length; i += 50) chunks.push(data.slice(i, i + 50));
  const results = [];
  for (const chunk of chunks) {
    const json = await request(session, url, {
      method: "POST",
      body: { data: chunk },
      accept: "application/vnd.api+json",
    });
    results.push(json);
  }
  return { added: data.length, batches: chunks.length, results };
}

async function addTracks(session, opts) {
  const playlist = await findPlaylist(session, requireOption(opts, "playlist"));
  const inputs = loadTrackInputs(opts);
  const resolved = [];
  const unresolved = [];
  for (const input of inputs) {
    const result = await resolveTrack(session, input);
    if (result.ok) resolved.push(result.track);
    else unresolved.push(result);
  }
  if (unresolved.length) return { ok: false, playlist, resolved, unresolved };
  let tracks = resolved;
  if (!opts["allow-duplicates"]) {
    const existing = new Set((await getPlaylistTracks(session, playlist.uuid, 500)).map((track) => track.id));
    tracks = resolved.filter((track) => !existing.has(track.id));
  }
  const data = tracks.map((track) => ({ id: track.id, type: "tracks" }));
  if (!data.length) {
    return { ok: true, playlist, requested: resolved.length, added: 0, skippedDuplicates: resolved.length, tracks: [], write: { skipped: true } };
  }
  const write = await addRelationship(session, `https://openapi.tidal.com/v2/playlists/${playlist.uuid}/relationships/items`, data, opts);
  return { ok: true, playlist, requested: resolved.length, added: tracks.length, skippedDuplicates: resolved.length - tracks.length, tracks, write };
}

async function favoriteTracks(session, opts) {
  if (!session.userId) throw new Error("session token does not include a user id");
  const inputs = loadTrackInputs(opts);
  const resolved = [];
  const unresolved = [];
  for (const input of inputs) {
    const result = await resolveTrack(session, input);
    if (result.ok) resolved.push(result.track);
    else unresolved.push(result);
  }
  if (unresolved.length) return { ok: false, resolved, unresolved };
  const data = resolved.map((track) => ({ id: track.id, type: "tracks" }));
  const write = await addRelationship(session, `https://openapi.tidal.com/v2/userCollections/${session.userId}/relationships/tracks`, data, opts);
  return { ok: true, favorited: resolved.length, tracks: resolved, write };
}

async function favoritePlaylist(session, opts) {
  if (!session.userId) throw new Error("session token does not include a user id");
  const playlist = await findPlaylist(session, requireOption(opts, "playlist"));
  const data = [{ id: playlist.uuid, type: "playlists" }];
  const write = await addRelationship(session, `https://openapi.tidal.com/v2/userCollections/${session.userId}/relationships/playlists`, data, opts);
  return { ok: true, playlist, write };
}

async function main() {
  const { command, opts } = parseArgs(process.argv);
  if (!command || command === "help" || opts.help) usage(0);
  const session = getSession();
  if (command === "session") {
    output({
      ok: true,
      source: session.source,
      userId: session.userId || null,
      countryCode: session.countryCode,
      expiresAt: session.expiresAt,
      scopes: session.scopes,
      refreshTokenPresent: session.refreshTokenPresent,
    }, opts);
  } else if (command === "playlists") {
    output(await listPlaylists(session, { filter: opts.filter, limit: opts.limit || DEFAULT_LIMIT }), opts);
  } else if (command === "playlist") {
    const playlist = await findPlaylist(session, requireOption(opts, "playlist"));
    const tracks = await getPlaylistTracks(session, playlist.uuid, opts.limit || DEFAULT_LIMIT);
    output({ ...playlist, numberOfTracks: tracks.length, tracks }, opts);
  } else if (command === "search-tracks") {
    const items = await searchTracks(session, requireOption(opts, "query"), opts.limit || 10);
    output(items.map((item) => simplifyTrack(item)), opts);
  } else if (command === "create-playlist") {
    output(await createPlaylist(session, opts), opts);
  } else if (command === "update-playlist") {
    output(await updatePlaylist(session, opts), opts);
  } else if (command === "add-tracks") {
    output(await addTracks(session, opts), opts);
  } else if (command === "favorite-tracks") {
    output(await favoriteTracks(session, opts), opts);
  } else if (command === "favorite-playlist") {
    output(await favoritePlaylist(session, opts), opts);
  } else {
    usage(1);
  }
}

main().catch((error) => {
  console.error(`tidal-control: ${error.message.replace(/[A-Za-z0-9_-]{80,}/g, "[redacted]")}`);
  process.exit(1);
});
