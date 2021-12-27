use super::app::App;
use crate::schema::domains;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Queryable, Serialize, Identifiable, Associations)]
#[belongs_to(App)]
pub struct Domain {
	pub id: i32,
	pub domain: String,
	pub verified: bool,
	pub app_id: i32,
}

#[derive(Clone, Deserialize, Debug, Insertable)]
#[table_name = "domains"]
pub struct NewDomain {
	pub domain: String,
	#[serde(skip_deserializing)]
	pub verified: bool,
	#[serde(skip_deserializing)]
	pub app_id: i32,
}
