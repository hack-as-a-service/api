use crate::schema::whitelist;

#[derive(Queryable, Insertable)]
#[table_name = "whitelist"]
pub struct WhitelistEntry {
	pub slack_user_id: String,
}
