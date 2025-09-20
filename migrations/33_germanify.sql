ALTER TABLE anträge RENAME TO antraege;
ALTER TABLE antraege RENAME COLUMN created_at TO erstellt_am;
ALTER TABLE antraege RENAME COLUMN begründung TO begruendung;

ALTER TABLE sitzungen RENAME COLUMN location TO ort;
ALTER TABLE sitzungen RENAME COLUMN kind TO typ;
ALTER TABLE sitzungen RENAME COLUMN legislative_period_id TO legislatur_periode_id;

ALTER TABLE legislative_period RENAME TO legislatur_perioden;

ALTER TABLE tops RENAME COLUMN kind to typ;

ALTER TABLE person RENAME COLUMN full_name to name;
