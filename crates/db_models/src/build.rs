use crate::app::App;
use crate::schema::builds;
use chrono::NaiveDateTime;
use serde::Serialize;

#[derive(Clone, Debug, Queryable, Serialize, Identifiable, Associations)]
#[belongs_to(App)]
pub struct Build {
	pub id: i32,
	pub started_at: NaiveDateTime,
	pub ended_at: Option<NaiveDateTime>,
	pub events: Vec<String>,
	pub app_id: i32,
}

#[derive(Clone, Insertable, Debug)]
#[table_name = "builds"]
pub struct NewBuild {
	pub app_id: i32,
}
