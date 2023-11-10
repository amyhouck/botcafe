-- Add migration script here
DROP TABLE guild_settings;

CREATE TABLE guild_settings (
    id INTEGER PRIMARY KEY NOT NULL,
    guild_id INTEGER NOT NULL,
    feed_settings_required_roleid INTEGER NOT NULL
)