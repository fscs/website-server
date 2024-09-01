INSERT INTO sitzungen (id, datetime, location, kind) 
VALUES ('ba788d36-4798-408b-8dd1-102095ae2d6d', '2024-09-10T12:30:00+2:00', 'ein uni raum', 'vv');

INSERT INTO tops (id, name, sitzung_id, weight, kind, inhalt)
    VALUES 
        ('91e12cf2-a773-4c8d-a418-8cf68478db43', 'no', 'ba788d36-4798-408b-8dd1-102095ae2d6d', 4, 'normal', null);
