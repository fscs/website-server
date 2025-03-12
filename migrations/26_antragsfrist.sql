alter table sitzungen 
    add COLUMN antragsfrist timestamptz
    NOT Null DEFAULT CURRENT_TIMESTAMP;
UPDATE sitzungen
    SET antragsfrist = datetime - INTERVAL '3 days';
alter table anträge
    add created_at timestamptz
    NOT Null DEFAULT '1970-01-01T00:00:00Z';
