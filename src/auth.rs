use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};

pub fn verify_password(password: &str, hashed_password: &str) -> bool {
    let argon2 = Argon2::default();

    let stored_hash = PasswordHash::new(hashed_password).expect("stored hash is in invalid format");

    argon2
        .verify_password(password.as_bytes(), &stored_hash)
        .is_ok()
}

pub fn generate_hash(password: &str) -> String {
    let argon2 = Argon2::default();
    let salt = SaltString::generate(OsRng);

    argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("error whilst hashung password")
        .to_string()
}
