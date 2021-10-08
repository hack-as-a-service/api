use chrono::Utc;
use db_models::{NewOauthDeviceRequest, OauthDeviceRequest};
use diesel::prelude::*;
use rocket::{
    form::{Form, Strict},
    http::Status,
    serde::json::Json,
};
use serde::Serialize;

use crate::{
    utils::oauth_device::{generate_device_code, generate_user_code},
    DbConn,
};
use diesel::result::{DatabaseErrorKind::ForeignKeyViolation, Error::DatabaseError};

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

#[derive(FromForm)]
pub struct AccessTokenRequest {
    grant_type: String,
    device_code: String,
    client_id: String,
}

#[derive(Serialize)]
#[serde(tag = "error")]
#[serde(rename_all = "snake_case")]
pub enum AccessTokenErrorResponseType {
    AuthorizationPending,
    ExpiredToken,
    AccessDenied,
    UnsupportedGrantType,
    InvalidRequest,
    ServerError,
}

#[derive(Responder)]
#[response(status = 400)]
pub struct AccessTokenErrorResponse(Json<AccessTokenErrorResponseType>);

impl AccessTokenErrorResponse {
    fn new(t: AccessTokenErrorResponseType) -> Self {
        Self(Json(t))
    }
}

#[derive(Serialize)]
pub struct AccessTokenResponse {
    access_token: String,
}

#[post("/oauth/device_authorization", data = "<request>")]
pub async fn device_authorization(
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

#[post("/oauth/token", data = "<request>")]
pub async fn token(
    request: Form<Strict<AccessTokenRequest>>,
    conn: DbConn,
) -> Result<Json<AccessTokenResponse>, AccessTokenErrorResponse> {
    if request.grant_type == "urn:ietf:params:oauth:grant-type:device_code" {
        conn.run(move |c| {
            use db_models::schema::oauth_device_requests::dsl::{
                device_code, oauth_app_id, oauth_device_requests, token_retrieved,
            };

            let req = oauth_device_requests
                .filter(
                    device_code
                        .eq(&request.device_code)
                        .and(oauth_app_id.eq(&request.client_id)),
                )
                .first::<OauthDeviceRequest>(c)
                .map_err(|_| {
                    // AccessTokenErrorResponse(Json(AccessTokenErrorResponseType::InvalidRequest))
                    AccessTokenErrorResponse::new(AccessTokenErrorResponseType::InvalidRequest)
                })?;

            if req.token_retrieved {
                return Err(AccessTokenErrorResponse::new(
                    AccessTokenErrorResponseType::InvalidRequest,
                ));
            }

            if req.expires_at.lt(&Utc::now().naive_utc()) {
                return Err(AccessTokenErrorResponse::new(
                    AccessTokenErrorResponseType::ExpiredToken,
                ));
            }

            if req.access_denied {
                return Err(AccessTokenErrorResponse::new(
                    AccessTokenErrorResponseType::AccessDenied,
                ));
            }

            match req.token {
                Some(access_token) => {
                    println!("hi");
                    diesel::update(oauth_device_requests)
                        .set(token_retrieved.eq(true))
                        .execute(c)
                        .map_err(|_| {
                            AccessTokenErrorResponse::new(AccessTokenErrorResponseType::ServerError)
                        })?;

                    Ok(Json(AccessTokenResponse { access_token }))
                }
                None => Err(AccessTokenErrorResponse::new(
                    AccessTokenErrorResponseType::AuthorizationPending,
                )),
            }
        })
        .await
    } else {
        return Err(AccessTokenErrorResponse(Json(
            AccessTokenErrorResponseType::UnsupportedGrantType,
        )));
    }
}
