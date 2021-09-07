use super::team::Team;
use crate::schema::apps;
use chrono::NaiveDateTime;
use serde::Serialize;

#[derive(Debug, Queryable, Serialize, Identifiable, Associations)]
#[belongs_to(Team)]
pub struct App {
    id: i32,
    created_at: NaiveDateTime,
    slug: String,
    team_id: i32,
    enabled: bool,
    #[serde(skip_serializing)]
    container_id: Option<String>,
}
