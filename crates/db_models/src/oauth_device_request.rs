use super::{OauthApp, Token};
use crate::schema::oauth_device_requests;
use chrono::NaiveDateTime;

#[derive(Queryable, Identifiable, Debug, Associations)]
#[belongs_to(OauthApp)]
#[belongs_to(Token, foreign_key = "token")]
pub struct OauthDeviceRequest {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub oauth_app_id: String,
    pub token: Option<String>,
    pub device_code: String,
    pub user_code: String,
}

#[derive(Insertable, Debug)]
#[table_name = "oauth_device_requests"]
pub struct NewOauthDeviceRequest {
    pub oauth_app_id: String,
    pub device_code: String,
    pub user_code: String,
}
