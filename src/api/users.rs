use rocket::serde::json::Json;

use crate::auth::HaaSUser;
use crate::models::user::User;

#[get("/users/me")]
pub fn me(user: HaaSUser) -> Json<User> {
    Json(user.0)
}
