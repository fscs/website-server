alter table tops 
    drop constraint tops_sitzung_id_fkey,
    add constraint tops_sitzung_id_fkey 
    foreign key(sitzung_id) references sitzungen(id) on delete cascade;
