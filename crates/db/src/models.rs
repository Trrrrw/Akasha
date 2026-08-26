use akasha_application::search::{TextQuery, TextQueryGroup};
use sea_orm::{
    Condition, ExprTrait,
    entity::prelude::*,
    sea_query::{Expr, Func, LikeExpr},
};

/// 为一个已解析文本查询构造跨字段、字面量且 ASCII 大小写不敏感的条件
pub(crate) fn text_query_condition(query: &TextQuery, fields: &[Expr]) -> Condition {
    let mut conditions = Condition::all();

    for group in &query.groups {
        let mut group_condition = Condition::any();
        for alternative in &group.alternatives {
            let pattern = escaped_contains_pattern(&alternative.to_lowercase());
            for field in fields {
                group_condition = group_condition.add(
                    Func::lower(Func::if_null(field.clone(), ""))
                        .like(LikeExpr::new(pattern.clone()).escape('\\')),
                );
            }
        }
        if group.excluded {
            group_condition = group_condition.not();
        }
        conditions = conditions.add(group_condition);
    }

    conditions
}

/// 为一个字面量关键词构造跨字段包含条件
pub(crate) fn literal_contains_condition(value: String, fields: &[Expr]) -> Condition {
    text_query_condition(
        &TextQuery {
            groups: vec![TextQueryGroup {
                excluded: false,
                alternatives: vec![value],
            }],
        },
        fields,
    )
}

/// 将用户文本转换为不会暴露 LIKE 通配符语义的包含模式
fn escaped_contains_pattern(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('%');
    for ch in value.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped.push('%');
    escaped
}

/// 审计日志中记录的操作主体类型
#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "lowercase"
)]
pub enum AuditLogActorType {
    User,
    Worker,
    System,
}

/// SeaORM 中保存的 worker 同步阶段
#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum WorkerPhase {
    #[sea_orm(string_value = "initial_backfill")]
    InitialBackfill,

    #[sea_orm(string_value = "incremental")]
    Incremental,
}

impl WorkerPhase {
    /// 返回 worker 阶段稳定的数据库字符串值
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InitialBackfill => "initial_backfill",
            Self::Incremental => "incremental",
        }
    }
}

/// SeaORM 中保存的 worker 生命周期状态
#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
pub enum WorkerStatus {
    #[sea_orm(string_value = "idle")]
    Idle,

    #[sea_orm(string_value = "running")]
    Running,

    #[sea_orm(string_value = "failed")]
    Failed,
}

impl WorkerStatus {
    /// 返回 worker 状态稳定的数据库字符串值
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Failed => "failed",
        }
    }
}
