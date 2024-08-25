alter table sitzungen drop column name;

create type sitzungtype as ENUM (
    'normal',
    'vv',
    'wahlvv',
    'ersatz',
    'konsti',
    'dringlichkeit'
);

alter table sitzungen add column sitzung_type sitzungtype not NULL DEFAULT 'normal';
