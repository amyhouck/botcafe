-- Add migration script here
CREATE TABLE heycafe_feeds (
    id INTEGER PRIMARY KEY NOT NULL,
    guild_id INTEGER NOT NULL,
    feed_type TEXT NOT NULL,
    channel_id INTEGER NOT NULL,
    heycafe_id TEXT NOT NULL,
    last_post_id TEXT NOT NULL,
    mention_role_id INTEGER NOT NULL,
    tag_id TEXT NOT NULL
)
