INSERT INTO sitzungen (id, datum, location, sitzung_type) 
VALUES ('ba788d36-4798-408b-8dd1-102095ae2d6d', '2024-09-10T12:30:00+2:00', 'ein uni raum', 'vv');

INSERT INTO tops (name, sitzung_id, weight, top_type, inhalt)
    VALUES 
        ('no', 'ba788d36-4798-408b-8dd1-102095ae2d6d', 4, 'normal', null),
        ('nosss', 'ba788d36-4798-408b-8dd1-102095ae2d6d', 42, 'sonstiges', null);
