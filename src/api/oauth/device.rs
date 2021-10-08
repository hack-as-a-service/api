use crate::{
    api::oauth::DeviceAuthorizationResponse,
    utils::oauth_device::{generate_device_code, generate_user_code},
    DbConn,
};
use db_models::NewOauthDeviceRequest;
use diesel::{
    prelude::*,
    result::{DatabaseErrorKind::ForeignKeyViolation, Error::DatabaseError},
};
use rocket::{
    form::{Form, Strict},
    http::Status,
    serde::json::Json,
};

use crate::api::oauth::DeviceAuthorizationRequest;

#[post("/oauth/device_authorization", data = "<request>")]
pub async fn create_device_authorization(
    request: Form<Strict<DeviceAuthorizationRequest>>,
    conn: DbConn,
) -> Result<Json<DeviceAuthorizationResponse>, Status> {
    conn.run(move |c| {
        use db_models::schema::oauth_device_requests::dsl::oauth_device_requests;

        let user_code = generate_user_code();
        let device_code = generate_device_code();

        diesel::insert_into(oauth_device_requests)
            .values(&NewOauthDeviceRequest {
                oauth_app_id: &request.client_id,
                device_code: &device_code,
                user_code: &user_code,
            })
            .execute(c)
            .map_err(|e| {
                if let DatabaseError(ForeignKeyViolation, _) = e {
                    Status::BadRequest
                } else {
                    Status::InternalServerError
                }
            })?;

        Ok(Json(DeviceAuthorizationResponse {
            device_code,
            user_code,
            verification_uri: String::from("https://hackclub.app"),
            expires_in: Some(900),
        }))
    })
    .await
}
