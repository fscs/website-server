alter table antragsstellende 
    drop constraint antragsstellende_antrags_id_fkey,
    add constraint antragsstellende_antrags_id_fkey 
    foreign key(antrags_id) references anträge(id) on delete cascade;

alter table antragsstellende 
    drop constraint antragsstellende_person_id_fkey,
    add constraint antragsstellende_person_id_fkey 
    foreign key(antrags_id) references anträge(id) on delete cascade;
