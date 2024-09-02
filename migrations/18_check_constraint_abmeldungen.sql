alter table abmeldungen 
    add check (anfangsdatum <= ablaufdatum);
