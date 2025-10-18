use std::crypto::*;


#[inline]
fn hash(password: &String) -> Result<String, Error> {
    crypto::hash_password(password)
}

#[inline]
fn verify(password: &String, hash: &String) -> Result<bool, Error> {
    crypto::verify_password(password, hash)
}

