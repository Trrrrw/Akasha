use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct GitHubCallbackQuery {
    pub(super) state: String,
    pub(super) code: String,
}
