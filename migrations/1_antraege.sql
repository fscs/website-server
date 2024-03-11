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

create table if not exists tops (
    id uuid primary key default gen_random_uuid(),
    name text not null,
    inhalt jsonb not null
);

create table if not exists antragstop (
    antrag_id uuid,
    top_id uuid,
    foreign key(antrag_id) references anträge(id),
    foreign key(top_id) references tops(id),
    primary key (antrag_id, top_id)
);

create table if not exists sitzungen (
    id uuid primary key default gen_random_uuid(),
    datum date not null,
    name text not null
);

create table if not exists sitzungstop (
    sitzung_id uuid,
    top_id uuid,
    foreign key(sitzung_id) references sitzungen(id),
    foreign key(top_id) references tops(id),
    primary key (sitzung_id, top_id)
);
