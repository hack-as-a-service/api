use rand::prelude::*;

use super::token::generate_token;

pub fn generate_user_code() -> String {
    let mut rng = thread_rng();

    format!("{:06}", rng.gen_range(0..1000000))
}

pub fn generate_device_code() -> String {
    generate_token()
}
