use chrono::{DateTime, FixedOffset, SecondsFormat, Utc};
use serde::Serialize;
use utoipa::ToSchema;

/// 将一个确定时间点统一序列化为精确到秒的 UTC RFC 3339 时间戳
pub fn utc_timestamp(value: DateTime<FixedOffset>) -> String {
    value
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// 将可选的站内资源相对路径转换为公开绝对地址
pub fn public_asset_url(asset_base_url: &str, value: Option<String>) -> Option<String> {
    value.map(|value| {
        if value.starts_with('/') && !value.starts_with("//") {
            // 内置游戏资源同时提供 WebP，避免浏览器解码 AVIF 失败后再请求
            let path = if value.starts_with("/assets/games/") {
                value
                    .strip_suffix(".avif")
                    .map(|path| format!("{path}.webp"))
                    .unwrap_or(value)
            } else {
                value
            };
            format!("{asset_base_url}{path}")
        } else {
            value
        }
    })
}

/// 递归把 JSON 中的站内资源路径转换为公开绝对地址
pub fn public_asset_json(asset_base_url: &str, value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(path) if path.starts_with('/') && !path.starts_with("//") => {
            serde_json::Value::String(
                public_asset_url(asset_base_url, Some(path)).expect("资源路径应存在"),
            )
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

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::{public_asset_json, public_asset_url, utc_timestamp};

    #[test]
    fn defaults_bundled_images_to_webp_without_rewriting_other_assets() {
        for (path, expected) in [
            (
                "/assets/games/ys/cover.avif",
                "https://assets.example/assets/games/ys/cover.webp",
            ),
            (
                "/assets/games/sr/icon-64.avif",
                "https://assets.example/assets/games/sr/icon-64.webp",
            ),
            (
                "/assets/games/ys/imaginarium-theater.png",
                "https://assets.example/assets/games/ys/imaginarium-theater.png",
            ),
            (
                "/assets/game-data/example.avif",
                "https://assets.example/assets/game-data/example.avif",
            ),
            (
                "https://external.example/cover.avif",
                "https://external.example/cover.avif",
            ),
            (
                "//external.example/cover.avif",
                "//external.example/cover.avif",
            ),
        ] {
            assert_eq!(
                public_asset_url("https://assets.example", Some(path.to_owned())).as_deref(),
                Some(expected)
            );
        }
        assert_eq!(public_asset_url("https://assets.example", None), None);
        let value = public_asset_json(
            "https://assets.example",
            serde_json::json!({
                "images": ["/assets/games/zzz/icon.avif", null]
            }),
        );
        assert_eq!(
            value["images"][0],
            "https://assets.example/assets/games/zzz/icon.webp"
        );
        assert!(value["images"][1].is_null());
    }

    #[test]
    fn serializes_timestamp_as_utc_to_second_precision() {
        let value = DateTime::parse_from_rfc3339("2026-07-01T11:00:00.123456+08:00")
            .expect("test timestamp should be valid");

        assert_eq!(utc_timestamp(value), "2026-07-01T03:00:00Z");
    }
}
