use crate::schema::teams;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Queryable, Serialize, Identifiable)]
pub struct Team {
	pub id: i32,
	#[serde(skip_serializing)]
	pub created_at: NaiveDateTime,
	pub name: Option<String>,
	pub avatar: Option<String>,
	pub personal: bool,
	pub slug: String,
	pub invite: String,
}

#[derive(Clone, Debug, Insertable, Deserialize)]
#[table_name = "teams"]
pub struct NewTeam {
	pub name: Option<String>,
	pub avatar: Option<String>,
	#[serde(skip_deserializing)]
	pub personal: bool,
	pub slug: String,
}

#[derive(Clone, Debug, AsChangeset, Deserialize)]
#[table_name = "teams"]
pub struct UpdatedTeam {
	pub name: Option<String>,
	pub slug: Option<String>,
	pub avatar: Option<String>,
}
