#!/usr/bin/env node
"use strict";

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const packageRoot = path.resolve(__dirname, "..");
const repoRoot = path.resolve(packageRoot, "../..");

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

function exists(relativePath) {
  return fs.existsSync(path.join(repoRoot, relativePath));
}

function fail(message) {
  throw new Error(message);
}

function assert(condition, message) {
  if (!condition) fail(message);
}

function listFiles(dir) {
  if (!fs.existsSync(dir)) return [];
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if ([".wrangler", ".git"].includes(entry.name)) return [full];
      if (entry.name === "node_modules") return [];
      return listFiles(full);
    }
    return [full];
  });
}

const packageJson = JSON.parse(read("packages/arcwell-garderobe/package.json"));
const readme = read("packages/arcwell-garderobe/README.md");
const indexTs = read("packages/arcwell-garderobe/src/index.ts");
const authTs = read("packages/arcwell-garderobe/src/auth.ts");
const functionalityDocs = read("docs/functionality-and-packages.md");
const liveDocs = read("docs/live-e2e-testing.md");
const allPackageFiles = listFiles(packageRoot).map((file) => path.relative(packageRoot, file));
const textPackageFiles = allPackageFiles.filter((file) =>
  /\.(?:md|ts|js|mjs|json|jsonc|sql|gitignore)$/.test(file) &&
  file !== "package-lock.json" &&
  file !== "scripts/severe-integration-tests.mjs"
);

/*
CLAIM: Arcwell can vendor Garderobe as a separate remote MCP package without
copying secrets/private inventory or teaching hosts to bypass OAuth, weather
fallbacks, or prompt-injection boundaries.
PRECONDITIONS: Tests run against this repository checkout only; they do not call
the live adjacent Garderobe deployment or touch production wardrobe data.
POSTCONDITIONS: A failure names the exact integration boundary that was weakened.
ORACLE: Static package/docs invariants that would be violated by common bad
integrations: copied seed SQL, copied local secrets, removed OAuth/DCR/PKCE,
missing host privacy language, or unsafe wardrobe metadata instructions.
SEVERITY: Severe for the integration layer because it targets auth, privacy,
prompt-injection, and production-data leakage failure modes.
*/

assert(packageJson.name === "arcwell-garderobe", "package must be named arcwell-garderobe");
assert(packageJson.private === true, "package must stay private until licensing/provenance is settled");
assert(packageJson.scripts?.test === "node scripts/severe-integration-tests.mjs", "npm test must run the severe boundary checks");
assert(!Object.keys(packageJson.scripts ?? {}).some((name) => /seed|remote.*test/i.test(name)), "package scripts must not expose private seed/live-remote test paths");

for (const forbidden of [".dev.vars"]) {
  assert(!exists(`packages/arcwell-garderobe/${forbidden}`), `${forbidden} must not be copied into arcwell-garderobe`);
}
assert(exists("packages/arcwell-garderobe/.gitignore"), "package .gitignore must protect local validation artifacts");
const gitignore = read("packages/arcwell-garderobe/.gitignore");
assert(/^node_modules\/$/m.test(gitignore), "package .gitignore must ignore locally installed node_modules");
assert(/^\.wrangler\/$/m.test(gitignore), "package .gitignore must ignore local Wrangler state");
assert(!allPackageFiles.some((file) => /^seed[\\/].*\.sql$/i.test(file)), "private seed SQL must not be copied into arcwell-garderobe");
assert(!allPackageFiles.some((file) => /severe-rotation-tests\.ts$/.test(file)), "live-remote severe test must not be copied into arcwell-garderobe");
assert(!allPackageFiles.some((file) => /seed-rotation\.ts$/.test(file)), "private rotation seed script must not be copied into arcwell-garderobe");

const privateInventoryMarkers = [
  "Drake",
  "Paraboot",
  "NB 990",
  "Lightweight oxford",
  "Chasseur",
  "SS26",
  "sprezzatura",
  "garderobe.chabot",
  "café marron",
  "Di Sondrio",
  "Akita",
  "Merino",
  "Clifford",
  "Reims",
  "Cerf",
  "California plaid",
  "Jungle Jacket",
  "Games Mk"
];
for (const file of textPackageFiles) {
  const content = fs.readFileSync(path.join(packageRoot, file), "utf8");
  for (const marker of privateInventoryMarkers) {
    assert(!content.toLowerCase().includes(marker.toLowerCase()), `${file} leaks private inventory marker: ${marker}`);
  }
}

assert(indexTs.includes("new OAuthProvider<Env>"), "Worker must keep the OAuth provider wrapper");
assert(indexTs.includes('clientRegistrationEndpoint: "/register"'), "Worker must keep DCR registration endpoint");
assert(indexTs.includes('tokenEndpoint: "/token"'), "Worker must keep OAuth token endpoint");
assert(indexTs.includes('apiRoute: "/mcp"'), "Worker must keep MCP behind the OAuth provider");
assert(indexTs.includes("allowPlainPKCE: false"), "Worker must reject plain PKCE for remote MCP auth");
assert(indexTs.includes('"wardrobe.read"') && indexTs.includes('"wardrobe.write"'), "Worker must expose explicit wardrobe scopes");
assert(authTs.includes("validateCsrf") && authTs.includes("WARDROBE_LOGIN_CODE"), "authorization flow must require CSRF and the single-user login code");

assert(/do not receive raw wardrobe inventory/i.test(readme), "README must say Arcwell memory/profile/wiki do not receive raw inventory by default");
assert(/untrusted data/i.test(readme), "README must mark wardrobe metadata as untrusted data");
assert(/weather lookup fails/i.test(readme) && /manual temperature\/conditions fallback/i.test(readme), "README must require manual fallback when weather lookup fails");
assert(/Do not sync private inventory into Arcwell memory\/profile\/wiki by default/i.test(readme), "README must forbid default private inventory sync");
assert(/OAuth 2\.1 with Dynamic\s+Client Registration/i.test(readme), "README must document OAuth 2.1 + DCR");
assert(/Do not put tokens or login codes in connector URLs/i.test(readme), "README must forbid URL token/login-code auth");

assert(/Package: `arcwell-garderobe`/.test(functionalityDocs), "functionality docs must include the arcwell-garderobe package");
assert(/private wardrobe source of truth/i.test(functionalityDocs), "functionality docs must define Garderobe as the private source of truth");
assert(/hostile wardrobe metadata/i.test(functionalityDocs), "functionality docs must cover hostile wardrobe metadata");
assert(/weather lookup fails/i.test(functionalityDocs), "functionality docs must cover weather failure fallback");
assert(/Private inventory sync is opt-in/i.test(liveDocs), "live E2E docs must state inventory sync is opt-in");
assert(/auth bypass/i.test(liveDocs) && /inventory leakage/i.test(liveDocs), "live E2E docs must include severe Garderobe abuse cases");

console.log("arcwell-garderobe severe integration tests passed");
