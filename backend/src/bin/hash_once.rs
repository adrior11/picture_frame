use std::env;

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};

fn main() {
    // Usage: cargo run --bin hash_once <SECRET>
    let secret = env::args().nth(1).expect("give the secret as CLI arg");
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(secret.as_bytes(), &salt)
        .unwrap()
        .to_string();
    println!("{hash}");
}
