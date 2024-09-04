alter table rolemapping
    drop constraint rollen_person_id_fkey,
    add constraint rolemapping_person_id_fkey 
    foreign key(person_id) references person(id) on delete cascade;
