ALTER TABLE rotations ADD COLUMN start_date TEXT;
ALTER TABLE rotations ADD COLUMN updated_at TEXT NOT NULL DEFAULT (datetime('now'));

CREATE INDEX IF NOT EXISTS idx_rotations_active ON rotations(active);
CREATE INDEX IF NOT EXISTS idx_rotation_slots_item ON rotation_slots(item_id);

CREATE TRIGGER IF NOT EXISTS rotations_au AFTER UPDATE ON rotations BEGIN
  UPDATE rotations SET updated_at = datetime('now') WHERE id = new.id;
END;
