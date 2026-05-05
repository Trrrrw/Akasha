use axum::http::{HeaderMap, StatusCode, header};
use rand::distr::{Alphanumeric, SampleString};
use std::sync::OnceLock;

static SETUP_TOKEN: OnceLock<String> = OnceLock::new();

pub fn init_setup_token() {
    let token = generate_setup_token();

    SETUP_TOKEN
        .set(token.clone())
        .expect("setup token should only be initialized once");

    tracing::warn!("admin setup token: {token}");
}

pub fn verify_setup_token(headers: &HeaderMap) -> Result<(), StatusCode> {
    let expected = SETUP_TOKEN.get().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let actual = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if actual == expected {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn generate_setup_token() -> String {
    Alphanumeric.sample_string(&mut rand::rng(), 48)
}
