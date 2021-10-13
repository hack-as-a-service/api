pub mod device;
pub use device::*;

use chrono::Utc;
use db_models::OauthDeviceRequest;
use diesel::prelude::*;
use rocket::{
	form::{Form, Strict},
	serde::json::Json,
};
use serde::Serialize;

use crate::DbConn;

#[derive(FromForm)]
pub struct AccessTokenRequest {
	grant_type: String,
	device_code: String,
	client_id: String,
}

#[derive(Serialize)]
#[serde(tag = "error")]
#[serde(rename_all = "snake_case")]
pub enum OauthErrorType {
	AuthorizationPending,
	ExpiredToken,
	AccessDenied,
	UnsupportedGrantType,
	InvalidRequest,
	InvalidGrant,
	ServerError,
	InvalidClient,
}

#[derive(Responder)]
#[response(status = 400)]
pub struct OauthError(Json<OauthErrorType>);

impl OauthError {
	fn new(t: OauthErrorType) -> Self {
		Self(Json(t))
	}
}

#[derive(Serialize)]
pub struct AccessTokenResponse {
	access_token: String,
}

#[post("/oauth/token", data = "<request>")]
pub async fn token(
	request: Form<Strict<AccessTokenRequest>>,
	conn: DbConn,
) -> Result<Json<AccessTokenResponse>, OauthError> {
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
				.map_err(|_| OauthError::new(OauthErrorType::InvalidRequest))?;

			if req.token_retrieved {
				return Err(OauthError::new(OauthErrorType::InvalidGrant));
			}

			if req.expires_at.lt(&Utc::now().naive_utc()) {
				return Err(OauthError::new(OauthErrorType::ExpiredToken));
			}

			if req.access_denied {
				return Err(OauthError::new(OauthErrorType::AccessDenied));
			}

			match req.token {
				Some(access_token) => {
					diesel::update(oauth_device_requests.find(req.id))
						.set(token_retrieved.eq(true))
						.execute(c)
						.map_err(|_| OauthError::new(OauthErrorType::ServerError))?;

					Ok(Json(AccessTokenResponse { access_token }))
				}
				None => Err(OauthError::new(OauthErrorType::AuthorizationPending)),
			}
		})
		.await
	} else {
		Err(OauthError(Json(OauthErrorType::UnsupportedGrantType)))
	}
}
