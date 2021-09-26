pub fn generate_token() -> String {
    let bytes: [u8; 16] = rand::random();

    hex::encode(bytes)
}
