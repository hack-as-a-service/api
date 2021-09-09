table! {
    apps (id) {
        id -> Int4,
        created_at -> Timestamp,
        slug -> Text,
        team_id -> Int4,
        enabled -> Bool,
        container_id -> Nullable<Text>,
    }
}

table! {
    team_users (team_id, user_id) {
        user_id -> Int4,
        team_id -> Int4,
    }
}

table! {
    teams (id) {
        id -> Int4,
        created_at -> Timestamp,
        name -> Nullable<Text>,
        avatar -> Nullable<Text>,
        personal -> Bool,
        slug -> Text,
    }
}

table! {
    tokens (token) {
        token -> Text,
        created_at -> Timestamp,
        expires_at -> Timestamp,
        user_id -> Int4,
    }
}

table! {
    users (id) {
        id -> Int4,
        created_at -> Timestamp,
        slack_user_id -> Text,
        name -> Text,
        avatar -> Nullable<Text>,
    }
}

table! {
    whitelist (slack_user_id) {
        slack_user_id -> Text,
    }
}

joinable!(apps -> teams (team_id));
joinable!(team_users -> teams (team_id));
joinable!(team_users -> users (user_id));
joinable!(tokens -> users (user_id));

allow_tables_to_appear_in_same_query!(apps, team_users, teams, tokens, users, whitelist,);
