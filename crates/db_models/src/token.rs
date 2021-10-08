use crate::schema::tokens;
use crate::user::User;
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
pub struct NewToken<'a> {
    pub token: &'a str,
    pub user_id: i32,
}
