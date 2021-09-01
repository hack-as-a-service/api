use crate::schema::teams;
use chrono::NaiveDateTime;
use regex::Regex;
use serde::{Deserialize, Serialize};

use rand::prelude::*;

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

/// Converts any string to a team-compatible slug
pub fn into_slug(text: &str, randomize: bool) -> String {
    lazy_static! {
        static ref INVALID_REGEX: Regex = Regex::new("[^a-z0-9 ]").unwrap();
    }

    let slug = INVALID_REGEX
        .replace_all(&text.to_lowercase(), "")
        .replace(" ", "-");

    if randomize {
        let mut rng = thread_rng();

        format!("{}-{:4}", slug, rng.gen_range(0..10000))
    } else {
        slug
    }
}
