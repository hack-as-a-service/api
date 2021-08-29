use serde::Serialize;

use super::{team::Team, user::User};
use crate::schema::team_users;

#[derive(Debug, Queryable, Serialize, Identifiable, Associations, Insertable)]
#[belongs_to(Team)]
#[belongs_to(User)]
#[primary_key(team_id, user_id)]
pub struct TeamUser {
    pub user_id: i32,
    pub team_id: i32,
}
