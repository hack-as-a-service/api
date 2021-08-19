use rocket::serde::json::Json;

use crate::auth::HaasUser;
use crate::models::user::User;

#[get("/users/me")]
pub fn me(user: HaasUser) -> Json<User> {
    Json(user.0)
}
