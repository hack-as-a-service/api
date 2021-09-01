use crate::schema::teams;
use chrono::NaiveDateTime;
use regex::Regex;
use serde::{Deserialize, Serialize};

lazy_static! {
    static ref SLUG_REGEX: Regex = Regex::new("^[a-z0-9\\-]+$").unwrap();
}

#[derive(Debug, Queryable, Serialize, Identifiable)]
pub struct Team {
    pub id: i32,
    #[serde(skip_serializing)]
    pub created_at: NaiveDateTime,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub personal: bool,
    pub slug: String,
}

#[derive(Debug, Insertable, Deserialize)]
#[table_name = "teams"]
pub struct NewTeam {
    pub name: Option<String>,
    pub avatar: Option<String>,
    #[serde(skip_deserializing)]
    pub personal: bool,
    pub slug: String,
}

pub fn validate_slug(slug: &str) -> bool {
    SLUG_REGEX.is_match(slug)
}
