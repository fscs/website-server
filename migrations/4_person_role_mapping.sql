create table if not exists rollen (
    person_id uuid not null,
    rolle text not null,
    anfangsdatum date not null,
    ablaufdatum date not null,
    foreign key(person_id) references person(id)
);
