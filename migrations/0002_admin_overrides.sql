CREATE TABLE IF NOT EXISTS member_overrides (
    discord_id TEXT PRIMARY KEY NOT NULL,
    nickname_raw TEXT NOT NULL,
    role_ids_json TEXT NOT NULL,
    badges_json TEXT NOT NULL
);

INSERT OR IGNORE INTO member_overrides
    (discord_id, nickname_raw, role_ids_json, badges_json)
VALUES
    (
        '376674641676206080',
        'Likholesye',
        '["1431744883671957576","1433909147987869728"]',
        '["staff"]'
    ),
    (
        '388732904605351942',
        'MrEka_',
        '["1431744883671957576","1433909147987869728"]',
        '["staff"]'
    ),
    (
        '959458266713321482',
        'TBEPDblHYA',
        '["1433750152316719211"]',
        '["yrod"]'
    );
