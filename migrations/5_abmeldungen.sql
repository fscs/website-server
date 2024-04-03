create table if not exists abmeldungen (
    person_id uuid not null,
    anfangsdatum date not null,
    ablaufdatum date not null,
    foreign key(person_id) references person(id)
);
