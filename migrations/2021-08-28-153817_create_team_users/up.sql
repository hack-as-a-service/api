-- Your SQL goes here
CREATE TABLE team_users (
    user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    team_id INTEGER NOT NULL REFERENCES teams (id) ON DELETE CASCADE,
    PRIMARY KEY (team_id, user_id)
)