alter table tops drop column inhalt;

alter table tops
    add column inhalt text not null default '';
