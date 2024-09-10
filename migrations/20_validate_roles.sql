alter table rollen rename to rolemapping;

create table if not exists roles (
    name text not null unique
);

insert into roles (name)
    values ('Rat');

alter table rolemapping
    add foreign key(rolle) references roles(name);
