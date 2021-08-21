use rocket::serde::json::Json;

use crate::models::user::User;

#[get("/users/me")]
pub fn me(user: User) -> Json<User> {
    Json(user)
}
