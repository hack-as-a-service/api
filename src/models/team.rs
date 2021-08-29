use crate::schema::teams;
use chrono::NaiveDateTime;
use serde::Serialize;

#[derive(Debug, Queryable, Serialize, Identifiable)]
pub struct Team {
    pub id: i32,
    #[serde(skip_serializing)]
    pub created_at: NaiveDateTime,
    pub name: String,
    pub avatar: Option<String>,
    pub personal: bool,
}

#[derive(Debug, Insertable)]
#[table_name = "teams"]
pub struct NewTeam {
    pub name: String,
    pub avatar: Option<String>,
    pub personal: bool,
}
