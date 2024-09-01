create type topkind as ENUM (
    'regularia',
    'bericht',
    'normal',
    'verschiedenes'
);

alter table tops rename column top_type to kind;

alter table tops alter column kind drop default;
alter table tops alter column kind type topkind using kind::text::topkind;
alter table tops alter column kind set default 'normal';
