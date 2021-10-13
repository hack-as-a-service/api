use crate::schema::whitelist;

#[derive(Queryable, Insertable, Identifiable)]
#[table_name = "whitelist"]
#[primary_key("slack_user_id")]
pub struct WhitelistEntry {
	pub slack_user_id: String,
}
