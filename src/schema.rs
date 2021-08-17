table! {
    users (id) {
        id -> Int4,
        created_at -> Timestamp,
        slack_user_id -> Text,
        name -> Nullable<Text>,
        avatar -> Nullable<Text>,
    }
}
