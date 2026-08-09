use crate::{features::auth::endpoints, state::AppState};
use utoipa_axum::{router::OpenApiRouter, routes};

/// 构建 GitHub OAuth 和令牌管理公开路由
pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(endpoints::github_login))
        .routes(routes!(endpoints::github_callback))
        .routes(routes!(endpoints::refresh_session))
        .routes(routes!(endpoints::logout))
        .routes(routes!(endpoints::current_user))
}
