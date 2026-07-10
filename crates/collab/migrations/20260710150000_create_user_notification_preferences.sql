CREATE TABLE user_notification_preferences (
    user_id BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    bypass_dnd_for_urgent BOOLEAN NOT NULL DEFAULT FALSE
);
