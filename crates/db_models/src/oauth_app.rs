use crate::schema::oauth_apps;

#[derive(Clone, Queryable, Debug, Identifiable)]
#[primary_key(client_id)]
pub struct OauthApp {
	pub client_id: String,
	pub name: String,
}
