create table legislative_period (
    id uuid primary key default gen_random_uuid(),
    name text not null
);

alter table sitzungen 
    add column legislative_period_id uuid
    references legislative_period(id);

-- Check if there are any sitzungen that need to be updated
WITH rows_to_update AS (
    SELECT 1
    FROM sitzungen
    WHERE legislative_period_id IS NULL
    LIMIT 1
),
new_legislative_period AS (
    -- Insert the legislative period only if there are rows to update
    INSERT INTO legislative_period (name)
    SELECT 'FSR 24/25'
    WHERE EXISTS (SELECT 1 FROM rows_to_update)
    RETURNING id
)
-- Update the sitzungen table if the legislative period was inserted
UPDATE sitzungen
SET legislative_period_id = (SELECT id FROM new_legislative_period)
WHERE legislative_period_id IS NULL;

alter table sitzungen 
    alter column legislative_period_id set not null;
