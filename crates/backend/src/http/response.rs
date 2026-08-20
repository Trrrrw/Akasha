use serde::Serialize;
use utoipa::ToSchema;

/// 将可选的站内资源相对路径转换为公开绝对地址
pub fn public_asset_url(asset_base_url: &str, value: Option<String>) -> Option<String> {
    value.map(|value| {
        if value.starts_with('/') && !value.starts_with("//") {
            format!("{asset_base_url}{value}")
        } else {
            value
        }
    })
}

/// 递归把 JSON 中的站内资源路径转换为公开绝对地址
pub fn public_asset_json(asset_base_url: &str, value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(path) if path.starts_with('/') && !path.starts_with("//") => {
            serde_json::Value::String(format!("{asset_base_url}{path}"))
        }
        serde_json::Value::Array(values) => serde_json::Value::Array(
            values
                .into_iter()
                .map(|value| public_asset_json(asset_base_url, value))
                .collect(),
        ),
        serde_json::Value::Object(values) => serde_json::Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, public_asset_json(asset_base_url, value)))
                .collect(),
        ),
        value => value,
    }
}

/// 不分页列表接口的统一响应外壳
#[derive(Serialize, ToSchema)]
#[schema(description = "列表数据响应")]
pub struct ListResponse<T> {
    /// 列表长度
    pub total: u64,
    /// 列表
    pub items: Vec<T>,
}

/// 分页列表接口的统一响应外壳
#[derive(Serialize, ToSchema)]
#[schema(description = "分页数据响应")]
pub struct PageResponse<T, M = ()> {
    /// 符合查询条件的数目
    pub total: u64,
    /// 获取数量
    pub limit: u64,
    /// 偏移
    pub offset: u64,
    /// 数目 <= limit 的条目
    pub items: Vec<T>,
    /// 额外上下文
    pub meta: M,
}

/// 统一错误响应体
#[derive(Serialize, ToSchema)]
#[schema(description = "接口错误响应")]
pub struct ErrorResponse {
    /// 错误信息
    message: String,
}

impl ErrorResponse {
    /// 为客户端可见消息创建标准错误响应体
    pub fn new(message: String) -> Self {
        Self { message }
    }
}
