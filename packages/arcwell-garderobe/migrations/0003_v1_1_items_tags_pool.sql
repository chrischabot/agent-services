PRAGMA foreign_keys = OFF;

ALTER TABLE items ADD COLUMN tags TEXT;

UPDATE items SET temp_min_c = 5, temp_max_c = 20
WHERE category = 'knitwear' AND subcategory = 'rugby';

UPDATE category_config
SET wears_per_cycle = 3
WHERE category = 'knitwear';
INSERT OR IGNORE INTO category_config(category, launders, cooldown_days, wears_per_cycle)
VALUES ('knitwear', 1, 7, 3);

DROP TRIGGER IF EXISTS items_ai;
DROP TRIGGER IF EXISTS items_ad;
DROP TRIGGER IF EXISTS items_au;
DROP TABLE IF EXISTS items_fts;

CREATE VIRTUAL TABLE items_fts USING fts5(
  name,
  colour,
  pattern,
  fabric,
  brand,
  subcategory,
  notes,
  source_detail,
  aliases,
  tags,
  content='items',
  content_rowid='rowid',
  tokenize='unicode61'
);

CREATE TRIGGER items_ai AFTER INSERT ON items BEGIN
  INSERT INTO items_fts(rowid, name, colour, pattern, fabric, brand, subcategory, notes, source_detail, aliases, tags)
  VALUES (new.rowid, new.name, new.colour, new.pattern, new.fabric, new.brand, new.subcategory, new.notes, new.source_detail, new.aliases, new.tags);
END;

CREATE TRIGGER items_ad AFTER DELETE ON items BEGIN
  INSERT INTO items_fts(items_fts, rowid, name, colour, pattern, fabric, brand, subcategory, notes, source_detail, aliases, tags)
  VALUES ('delete', old.rowid, old.name, old.colour, old.pattern, old.fabric, old.brand, old.subcategory, old.notes, old.source_detail, old.aliases, old.tags);
END;

CREATE TRIGGER items_au AFTER UPDATE ON items BEGIN
  INSERT INTO items_fts(items_fts, rowid, name, colour, pattern, fabric, brand, subcategory, notes, source_detail, aliases, tags)
  VALUES ('delete', old.rowid, old.name, old.colour, old.pattern, old.fabric, old.brand, old.subcategory, old.notes, old.source_detail, old.aliases, old.tags);
  INSERT INTO items_fts(rowid, name, colour, pattern, fabric, brand, subcategory, notes, source_detail, aliases, tags)
  VALUES (new.rowid, new.name, new.colour, new.pattern, new.fabric, new.brand, new.subcategory, new.notes, new.source_detail, new.aliases, new.tags);
END;

INSERT INTO items_fts(items_fts) VALUES('rebuild');

CREATE UNIQUE INDEX IF NOT EXISTS idx_items_active_name_category
ON items(name, category)
WHERE deleted_at IS NULL;

PRAGMA foreign_keys = ON;
