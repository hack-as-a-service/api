use super::team::Team;
use crate::schema::apps;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Queryable, Serialize, Identifiable, Associations)]
#[belongs_to(Team)]
pub struct App {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub slug: String,
    pub team_id: i32,
    pub enabled: bool,
    #[serde(skip_serializing)]
    pub container_id: Option<String>,
}

#[derive(Insertable, Deserialize, Debug)]
#[table_name = "apps"]
pub struct NewApp {
    pub slug: String,
    #[serde(skip_deserializing)]
    pub team_id: i32,
}
