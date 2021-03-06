use chrono::NaiveDateTime;

use crate::schema::users;
use serde::Serialize;

#[derive(Clone, Debug, Queryable, Serialize, Identifiable)]
pub struct User {
	pub id: i32,
	#[serde(skip_serializing)]
	pub created_at: NaiveDateTime,
	pub slack_user_id: String,
	pub name: String,
	pub avatar: Option<String>,
}

#[derive(Clone, Debug, Insertable)]
#[table_name = "users"]
pub struct NewUser {
	pub slack_user_id: String,
	pub name: String,
	pub avatar: Option<String>,
}
