use chrono::NaiveDateTime;

use crate::schema::users;
use rocket::serde::Serialize;

#[derive(Debug, Queryable, Serialize, Identifiable)]
pub struct User {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub slack_user_id: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Debug, Insertable)]
#[table_name = "users"]
pub struct NewUser {
    pub slack_user_id: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
}
