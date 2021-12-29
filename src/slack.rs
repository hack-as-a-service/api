use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AccessTokenResponse {
	pub ok: bool,
	pub access_token: Option<String>,
	pub id_token: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct UserInfo {
	pub name: String,
	pub picture: String,
	#[serde(rename(deserialize = "https://slack.com/user_id"))]
	pub user_id: String,
}

/// Exchanges an authorization code for a Slack access token
pub async fn exchange_code(
	code: &str,
	client_id: &str,
	client_secret: &str,
	redirect_uri: &str,
) -> Option<AccessTokenResponse> {
	let client = reqwest::Client::new();

	let resp = client
		.post("https://slack.com/api/openid.connect.token")
		.form(&[
			("code", code),
			("client_id", client_id),
			("client_secret", client_secret),
			("redirect_uri", redirect_uri),
		])
		.send()
		.await
		.ok()?
		.json::<AccessTokenResponse>()
		.await
		.ok()?;

	Some(resp)
}

pub fn parse_id_token(token: &str) -> Result<UserInfo, ()> {
	let info = decode::<UserInfo>(
		token,
		// From https://slack.com/openid/connect/keys
		&DecodingKey::from_rsa_components("zQqzXfb677bpMKw0idKC5WkVLyqk04PWMsWYJDKqMUUuu_PmzdsvXBfHU7tcZiNoHDuVvGDqjqnkLPEzjXnaZY0DDDHvJKS0JI8fkxIfV1kNy3DkpQMMhgAwnftUiSXgb5clypOmotAEm59gHPYjK9JHBWoHS14NYEYZv9NVy0EkjauyYDSTz589aiKU5lA-cePG93JnqLw8A82kfTlrJ1IIJo2isyBGANr0YzR-d3b_5EvP7ivU7Ph2v5JcEUHeiLSRzIzP3PuyVFrPH659Deh-UAsDFOyJbIcimg9ITnk5_45sb_Xcd_UN6h5I7TGOAFaJN4oi4aaGD4elNi_K1Q", 	"AQAB"),
		&Validation::new(Algorithm::RS256),
	)
	.map_err(|_| ())?
	.claims;

	Ok(info)
}
