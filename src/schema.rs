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
        name -> Nullable<Text>,
        avatar -> Nullable<Text>,
    }
}

joinable!(team_users -> teams (team_id));
joinable!(team_users -> users (user_id));
joinable!(tokens -> users (user_id));

allow_tables_to_appear_in_same_query!(team_users, teams, tokens, users,);
