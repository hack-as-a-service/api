-- Your SQL goes here
CREATE TABLE invites (
    user_id INTEGER NOT NULL REFERENCES users (id),
    team_id INTEGER NOT NULL REFERENCES teams (id),
    PRIMARY KEY (team_id, user_id)
)
