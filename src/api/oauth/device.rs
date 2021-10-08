use crate::{
    api::oauth::{OauthError, OauthErrorType},
    auth::AuthUser,
    utils::{
        oauth_device::{generate_device_code, generate_user_code},
        token::generate_token,
    },
    DbConn,
};
use db_models::{NewOauthDeviceRequest, NewToken, OauthApp, OauthDeviceRequest};
use diesel::{
    dsl::{not, now},
    prelude::*,
    result::{
        DatabaseErrorKind::ForeignKeyViolation,
        Error::{self, DatabaseError, NotFound},
    },
};
use rocket::{
    form::{Form, Strict},
    http::Status,
    response::status::NoContent,
    serde::json::Json,
};
use serde::Serialize;

#[derive(FromForm)]
pub struct DeviceAuthorizationRequest {
    client_id: String,
}

#[derive(Serialize)]
pub struct DeviceAuthorizationResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: Option<i32>,
}

#[get("/oauth/device_authorizations/<user_code>/app_name")]
pub async fn device_authorization(
    _user: AuthUser,
    conn: DbConn,
    user_code: String,
) -> Result<String, Status> {
    conn.run(move |c| {
        use db_models::schema::oauth_apps::dsl::oauth_apps;
        use db_models::schema::oauth_device_requests::dsl::{
            self as oauth_device_request, oauth_device_requests,
        };

        let (_, app) = oauth_device_requests
            .filter(
                oauth_device_request::user_code.eq(&user_code).and(
                    oauth_device_request::expires_at
                        .gt(now)
                        .and(not(oauth_device_request::access_denied))
                        .and(oauth_device_request::token.is_null()),
                ),
            )
            .inner_join(oauth_apps)
            .first::<(OauthDeviceRequest, OauthApp)>(c)
            .map_err(|e| {
                if e == NotFound {
                    Status::NotFound
                } else {
                    Status::InternalServerError
                }
            })?;

        Ok(app.name)
    })
    .await
}

#[post("/oauth/device_authorizations/<user_code>/approve")]
pub async fn device_approve(
    user: AuthUser,
    conn: DbConn,
    user_code: String,
) -> Result<NoContent, Status> {
    conn.run(move |c| {
        use db_models::schema::oauth_device_requests::dsl::{
            self as oauth_device_request, oauth_device_requests,
        };
        use db_models::schema::tokens::dsl::tokens;

        let req = oauth_device_requests
            .filter(
                oauth_device_request::user_code.eq(&user_code).and(
                    oauth_device_request::expires_at
                        .gt(now)
                        .and(not(oauth_device_request::access_denied))
                        .and(oauth_device_request::token.is_null()),
                ),
            )
            .first::<OauthDeviceRequest>(c)
            .map_err(|e| {
                if e == NotFound {
                    Status::NotFound
                } else {
                    Status::InternalServerError
                }
            })?;

        Connection::transaction::<(), Error, _>(c, || {
            let token = generate_token();

            diesel::insert_into(tokens)
                .values(&NewToken {
                    user_id: user.id,
                    token: &token,
                })
                .execute(c)?;

            diesel::update(oauth_device_requests.find(req.id))
                .set(oauth_device_request::token.eq(&token))
                .execute(c)?;

            Ok(())
        })
        .map_err(|_| Status::InternalServerError)?;

        Ok(NoContent)
    })
    .await
}

#[post("/oauth/device_authorization", data = "<request>")]
pub async fn create_device_authorization(
    request: Form<Strict<DeviceAuthorizationRequest>>,
    conn: DbConn,
) -> Result<Json<DeviceAuthorizationResponse>, OauthError> {
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
                    OauthError::new(OauthErrorType::InvalidClient)
                } else {
                    OauthError::new(OauthErrorType::ServerError)
                }
            })?;

        Ok(Json(DeviceAuthorizationResponse {
            device_code,
            user_code,
            verification_uri: String::from("https://hackclub.app/auth/device"),
            expires_in: Some(900),
        }))
    })
    .await
}
