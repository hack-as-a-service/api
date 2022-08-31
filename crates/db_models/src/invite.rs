use serde::Serialize;
use crate::schema::team_users;
use crate::{team::Team, user::User};

#[derive(Clone, Debug, Queryable, Serialize, Identifiable, Associations, Insertable)]
#[belongs_to(Team)]
#[belongs_to(User)]
#[primary_key(team_id, user_id)]

pub struct Invite {
	pub user_id: i32,
	pub team_id: i32,
}
