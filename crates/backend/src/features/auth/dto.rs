use serde::Serialize;
use utoipa::ToSchema;

/// 刷新会话后返回的短期 access token 响应
#[derive(Serialize, ToSchema)]
#[schema(description = "短期访问令牌")]
pub(crate) struct TokenResponse {
    /// 用于 Authorization 请求头的访问令牌
    pub(super) access_token: String,
    /// 令牌类型，固定为 Bearer
    pub(super) token_type: &'static str,
    /// 访问令牌剩余有效时间，单位为秒
    pub(super) expires_in: u64,
}

/// 仅包含状态消息的认证操作响应
#[derive(Serialize, ToSchema)]
#[schema(description = "认证操作结果")]
pub(crate) struct MessageResponse {
    /// 操作结果消息
    pub(super) message: String,
}

/// bearer token 对应的当前用户公开资料
#[derive(Serialize, ToSchema)]
#[schema(description = "当前登录用户资料")]
pub(crate) struct MeResponse {
    /// 用户 ID
    pub(super) id: String,
    /// 用户展示名称
    pub(super) display_name: String,
    /// 用户头像地址
    pub(super) avatar_url: Option<String>,
    /// 用户所属权限组
    pub(super) groups: Vec<String>,
    /// 是否为管理员
    pub(super) is_admin: bool,
}
