CREATE TABLE attachments (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    filename text NOT NULL
);

CREATE TABLE attachment_mapping (
    attachment_id uuid,
    antrags_id uuid,
    foreign key(attachment_id) references attachments(id) on delete cascade,
    foreign key(antrags_id) references anträge(id) on delete cascade,
    primary key (attachment_id, antrags_id)
);
