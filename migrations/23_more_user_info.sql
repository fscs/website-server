alter table person
    drop constraint person_name_key;

alter table person
    rename name to last_name;
    
alter table person
    add column first_name text not null default '',
    add column user_name text not null default '',
    add column matrix_id text DEFAULT NULL;

update person set
    first_name = array_to_string(
        trim_array(
            regexp_split_to_array(last_name, ' '),
            1
        ),
        ' '
    ),
    last_name = split_part(last_name, ' ', -1);
