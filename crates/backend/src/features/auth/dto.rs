use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct TokenResponse {
    pub(super) access_token: String,
    pub(super) token_type: &'static str,
    pub(super) expires_in: u64,
}

#[derive(Serialize)]
pub(crate) struct MessageResponse {
    pub(super) message: String,
}

#[derive(Serialize)]
pub(crate) struct MeResponse {
    pub(super) id: String,
    pub(super) display_name: String,
    pub(super) avatar_url: Option<String>,
    pub(super) groups: Vec<String>,
    pub(super) is_admin: bool,
}
