PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS items (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  category TEXT NOT NULL CHECK (category IN
    ('shirt','trouser','outerwear','footwear','sock','belt','tie','scarf','accessory','knitwear','other')),
  subcategory TEXT,
  colour TEXT,
  pattern TEXT,
  fabric TEXT,
  brand TEXT,
  size TEXT,
  fit_notes TEXT,
  seasons TEXT,
  temp_min_c INTEGER,
  temp_max_c INTEGER,
  formality INTEGER,
  status TEXT NOT NULL DEFAULT 'active' CHECK (status IN
    ('active','breaking_in','benched','secondary','retired')),
  notes TEXT,
  source_detail TEXT,
  aliases TEXT,
  price REAL,
  currency TEXT DEFAULT 'GBP',
  acquired_date TEXT,
  link TEXT,
  ref_code TEXT,
  quantity INTEGER DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_items_cat ON items(category, status);
CREATE INDEX IF NOT EXISTS idx_items_temp ON items(temp_min_c, temp_max_c);
CREATE INDEX IF NOT EXISTS idx_items_ref ON items(ref_code);
CREATE INDEX IF NOT EXISTS idx_items_brand ON items(brand);

CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(
  name,
  colour,
  pattern,
  fabric,
  brand,
  subcategory,
  notes,
  source_detail,
  aliases,
  content='items',
  content_rowid='rowid',
  tokenize='unicode61'
);

CREATE TRIGGER IF NOT EXISTS items_ai AFTER INSERT ON items BEGIN
  INSERT INTO items_fts(rowid, name, colour, pattern, fabric, brand, subcategory, notes, source_detail, aliases)
  VALUES (new.rowid, new.name, new.colour, new.pattern, new.fabric, new.brand, new.subcategory, new.notes, new.source_detail, new.aliases);
END;

CREATE TRIGGER IF NOT EXISTS items_ad AFTER DELETE ON items BEGIN
  INSERT INTO items_fts(items_fts, rowid, name, colour, pattern, fabric, brand, subcategory, notes, source_detail, aliases)
  VALUES ('delete', old.rowid, old.name, old.colour, old.pattern, old.fabric, old.brand, old.subcategory, old.notes, old.source_detail, old.aliases);
END;

CREATE TRIGGER IF NOT EXISTS items_au AFTER UPDATE ON items BEGIN
  INSERT INTO items_fts(items_fts, rowid, name, colour, pattern, fabric, brand, subcategory, notes, source_detail, aliases)
  VALUES ('delete', old.rowid, old.name, old.colour, old.pattern, old.fabric, old.brand, old.subcategory, old.notes, old.source_detail, old.aliases);
  INSERT INTO items_fts(rowid, name, colour, pattern, fabric, brand, subcategory, notes, source_detail, aliases)
  VALUES (new.rowid, new.name, new.colour, new.pattern, new.fabric, new.brand, new.subcategory, new.notes, new.source_detail, new.aliases);
END;

CREATE TABLE IF NOT EXISTS suggestion_sets (
  id TEXT PRIMARY KEY,
  for_date TEXT NOT NULL,
  n_options INTEGER NOT NULL,
  context TEXT,
  resolved INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS suggestion_items (
  set_id TEXT NOT NULL REFERENCES suggestion_sets(id) ON DELETE CASCADE,
  option_no INTEGER NOT NULL,
  role TEXT NOT NULL,
  item_id TEXT NOT NULL REFERENCES items(id),
  PRIMARY KEY (set_id, option_no, role)
);
CREATE INDEX IF NOT EXISTS idx_sugg_item ON suggestion_items(item_id);
CREATE INDEX IF NOT EXISTS idx_sugg_date ON suggestion_sets(for_date);

CREATE TABLE IF NOT EXISTS wear_log (
  id TEXT PRIMARY KEY,
  item_id TEXT NOT NULL REFERENCES items(id),
  worn_date TEXT NOT NULL,
  outfit_ref TEXT,
  source TEXT NOT NULL DEFAULT 'confirmed' CHECK (source IN ('confirmed','rotation','manual')),
  notes TEXT
);
CREATE INDEX IF NOT EXISTS idx_wear_item ON wear_log(item_id, worn_date DESC);
CREATE INDEX IF NOT EXISTS idx_wear_date ON wear_log(worn_date DESC);

CREATE TABLE IF NOT EXISTS category_config (
  category TEXT PRIMARY KEY,
  launders INTEGER NOT NULL,
  cooldown_days INTEGER NOT NULL,
  wears_per_cycle INTEGER NOT NULL DEFAULT 1
);

INSERT OR IGNORE INTO category_config(category, launders, cooldown_days, wears_per_cycle) VALUES
  ('shirt', 1, 7, 1),
  ('trouser', 1, 7, 3),
  ('sock', 1, 7, 1),
  ('outerwear', 0, 1, 1),
  ('footwear', 0, 1, 1),
  ('belt', 0, 1, 1),
  ('tie', 0, 1, 1),
  ('scarf', 0, 1, 1),
  ('accessory', 0, 1, 1),
  ('knitwear', 1, 7, 2),
  ('other', 0, 1, 1);

CREATE TABLE IF NOT EXISTS rotations (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  season TEXT,
  days INTEGER NOT NULL,
  notes TEXT,
  active INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS rotation_slots (
  id TEXT PRIMARY KEY,
  rotation_id TEXT NOT NULL REFERENCES rotations(id) ON DELETE CASCADE,
  day_number INTEGER NOT NULL,
  role TEXT NOT NULL CHECK (role IN ('outerwear','shirt','trouser','sock','footwear','tie','scarf','belt','accessory')),
  item_id TEXT NOT NULL REFERENCES items(id),
  UNIQUE(rotation_id, day_number, role)
);

CREATE TABLE IF NOT EXISTS meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
