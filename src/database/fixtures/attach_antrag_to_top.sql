INSERT INTO sitzungen (id, datum, location, sitzung_type) 
    VALUES 
        ('ba788d36-4798-408b-8dd1-102095ae2d6d', '2024-09-10T12:30:00+2:00', 'ein uni raum', 'vv');
        
INSERT INTO person (id, name)
    VALUES 
        ('21e51525-2de9-4650-8538-383bb0845cf3', 'deine mutter');
        
INSERT INTO anträge (id, titel, antragstext, begründung) 
    VALUES 
        ('641d6bbe-990c-4ece-9e38-dd3cd0d77460', 'mein antrag', 'ich beschließe', 'also bin ich');

INSERT INTO antragsstellende (antrags_id, person_id) 
    VALUES 
        ('641d6bbe-990c-4ece-9e38-dd3cd0d77460', '21e51525-2de9-4650-8538-383bb0845cf3');

INSERT INTO tops (id, name, sitzung_id, weight, top_type, inhalt)
    VALUES 
        ('78d38fbf-b360-41ad-be0d-ddcffdd47bb2', 'no', 'ba788d36-4798-408b-8dd1-102095ae2d6d', 4, 'normal', null);

        
