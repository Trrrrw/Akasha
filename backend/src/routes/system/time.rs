use axum::Json;
use chrono::{FixedOffset, SecondsFormat, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use axum_mcp::mcp;

#[mcp]
#[utoipa::path(
    get,
    path = "/time",
    tag = "System",
    summary = "获取当前服务器时间",
    description = "返回服务器当前的东八区时间、UTC 时间、时区信息和 Unix 时间戳。",
    responses(
        (status = 200, body = TimeResponse)
    )
)]
pub async fn time() -> Json<TimeResponse> {
    let utc = Utc::now();
    let offset = FixedOffset::east_opt(8 * 3600).expect("valid timezone offset");
    let local = utc.with_timezone(&offset);

    Json(TimeResponse {
        local: local.to_rfc3339_opts(SecondsFormat::Secs, true),
        utc: utc.to_rfc3339_opts(SecondsFormat::Secs, true),
        timezone: "Asia/Shanghai".to_string(),
        utc_offset: "+08:00".to_string(),
        unix_timestamp: utc.timestamp(),
    })
}

#[derive(Serialize, ToSchema)]
#[schema(description = "服务器当前时间响应。")]
pub struct TimeResponse {
    /// 服务器本地时间，Asia/Shanghai，RFC3339 格式。
    local: String,
    /// UTC 时间，RFC3339 格式。
    utc: String,
    /// 本地时区名称。
    timezone: String,
    /// 本地时区相对 UTC 的偏移。
    utc_offset: String,
    /// Unix 时间戳，单位为秒。
    unix_timestamp: i64,
}
