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

joinable!(tokens -> users (user_id));

allow_tables_to_appear_in_same_query!(tokens, users,);
