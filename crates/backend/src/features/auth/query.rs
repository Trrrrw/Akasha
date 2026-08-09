use serde::Deserialize;
use utoipa::IntoParams;

/// GitHub OAuth 回调 URL 的查询参数
#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct GithubCallbackQuery {
    /// 登录开始时生成并写入 Cookie 的防伪值
    pub(super) state: String,
    /// GitHub 回调提供的一次性授权码
    pub(super) code: String,
}
