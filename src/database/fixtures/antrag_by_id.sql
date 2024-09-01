INSERT INTO person (id, name)
    VALUES 
        ('0f3107ac-745d-4077-8bbf-f9734cd66297', 'deine mutter'),
        ('5a5a134d-9345-4c36-a466-1c3bb806b240', 'ihre mutter'),
        ('51288f16-4442-4d7c-9606-3dce198b0601', 'meine mutter');

INSERT INTO anträge (id, titel, antragstext, begründung)
    VALUES
        ('46148231-87b0-4486-8043-c55038178518', 'Blumen für Valentin', 'get them', 'Valentin deserves them'),
        ('5c51d5c0-3943-4695-844d-4c47da854fac', 'blub', 'blub', 'bulabsb');

INSERT INTO antragsstellende (antrags_id, person_id)
    VALUES
        ('46148231-87b0-4486-8043-c55038178518', '5a5a134d-9345-4c36-a466-1c3bb806b240'),
        ('46148231-87b0-4486-8043-c55038178518', '51288f16-4442-4d7c-9606-3dce198b0601'),
        ('5c51d5c0-3943-4695-844d-4c47da854fac', '0f3107ac-745d-4077-8bbf-f9734cd66297');
