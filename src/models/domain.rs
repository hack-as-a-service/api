use super::app::App;
use crate::schema::domains;
use regex::Regex;
use serde::{Deserialize, Serialize};

lazy_static! {
    static ref DOMAIN_REGEX: Regex = Regex::new("^([A-Za-z0-9-]{1,63}\\.)+[A-Za-z]{2,6}$").unwrap();
}

pub fn validate_domain(domain: &str) -> bool {
    DOMAIN_REGEX.is_match(domain)
}

#[derive(Debug, Queryable, Serialize, Identifiable, Associations)]
#[belongs_to(App)]
pub struct Domain {
    pub id: i32,
    pub domain: String,
    pub verified: bool,
    pub app_id: i32,
}

#[derive(Deserialize, Debug, Insertable)]
#[table_name = "domains"]
pub struct NewDomain {
    pub domain: String,
    #[serde(skip_deserializing)]
    pub verified: bool,
    #[serde(skip_deserializing)]
    pub app_id: i32,
}
