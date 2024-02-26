create table if not exists anträge (
    id uuid primary key default gen_random_uuid(),
    titel text not null,
    antragstext text not null,
    begründung text not null
);

create table if not exists person (
    id uuid primary key default gen_random_uuid(),
    name text not null
);

create table if not exists antragsstellende (
    antrags_id uuid,
    person_id uuid,
    name text,
    foreign key(antrags_id) references anträge(id),
    foreign key(person_id) references person(id),
    primary key (antrags_id, person_id)
);