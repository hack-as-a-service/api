use super::user::User;
use crate::schema::tokens;
use chrono::NaiveDateTime;

#[derive(Debug, Queryable, Identifiable, Associations)]
#[primary_key(token)]
#[table_name = "tokens"]
#[belongs_to(User)]
pub struct Token {
    pub token: String,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub user_id: i32,
}

#[derive(Debug, Insertable)]
#[table_name = "tokens"]
pub struct NewToken {
    pub token: String,
    pub user_id: i32,
}

pub fn generate_token() -> String {
    let bytes: [u8; 16] = rand::random();

    hex::encode(bytes)
}
