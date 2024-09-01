alter table antragstop 
    drop constraint antragstop_antrag_id_fkey,
    add constraint antragstop_antrag_id_fkey 
    foreign key(antrag_id) references anträge(id) on delete cascade;
    
alter table antragstop 
    drop constraint antragstop_top_id_fkey,
    add constraint antragstop_top_id_fkey 
    foreign key(top_id) references tops(id) on delete cascade;
