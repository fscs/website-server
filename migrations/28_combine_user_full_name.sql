alter table person
    rename last_name to full_name;

update person set 
    full_name = first_name || ' ' || full_name;

alter table person
    drop column first_name;
