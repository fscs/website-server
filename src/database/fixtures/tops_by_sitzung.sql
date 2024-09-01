INSERT INTO sitzungen (id, datetime, location, kind) 
    VALUES 
        ('ba788d36-4798-408b-8dd1-102095ae2d6d', '2024-09-10T12:30:00+2:00', 'ein uni raum', 'vv'),
        ('5715e4a0-36d0-4069-a863-abc3e915475a', '2024-09-10T12:30:00+2:00', 'ein uni raum', 'vv');

INSERT INTO tops (id, name, sitzung_id, weight, kind, inhalt)
    VALUES 
        ('78d38fbf-b360-41ad-be0d-ddcffdd47bb2', 'no', 'ba788d36-4798-408b-8dd1-102095ae2d6d', 4, 'normal', null),
        ('949554e6-a753-4847-93b9-691ad58177c7', 'no', '5715e4a0-36d0-4069-a863-abc3e915475a', 4, 'normal', null),
        ('9cce6322-029a-498e-8385-c1f9644077a5', 'no', 'ba788d36-4798-408b-8dd1-102095ae2d6d', 4, 'normal', null),
        ('dba69c43-8374-4e75-8712-2e54734c88c3', 'no', '5715e4a0-36d0-4069-a863-abc3e915475a', 4, 'normal', null);
