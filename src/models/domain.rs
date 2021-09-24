use super::app::App;
use crate::schema::domains;
use serde::Serialize;

#[derive(Debug, Queryable, Serialize, Identifiable, Associations)]
#[belongs_to(App)]
pub struct Domain {
    pub id: i32,
    pub domain: String,
    pub verified: bool,
    pub app_id: i32,
}
