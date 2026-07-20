use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[schema(description = "列表数据响应")]
pub struct ListResponse<T> {
    /// 列表长度
    pub total: u64,
    /// 列表
    pub items: Vec<T>,
}

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

#[derive(Serialize, ToSchema)]
#[schema(description = "接口错误响应")]
pub struct ErrorResponse {
    /// 错误信息
    message: String,
}

impl ErrorResponse {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}
