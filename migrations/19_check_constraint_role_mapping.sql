alter table rollen 
    add check (anfangsdatum <= ablaufdatum);
