use crate::schema::whitelist;

#[derive(Clone, Queryable, Insertable)]
#[table_name = "whitelist"]
pub struct WhitelistEntry {
	pub slack_user_id: String,
}
