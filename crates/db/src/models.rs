use sea_orm::entity::prelude::*;

/// 已拆分为包含词和排除词的标题搜索条件
#[derive(Debug, Default)]
pub struct TitleQuery {
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
}

impl TitleQuery {
    /// 将空格分隔、支持减号排除的标题查询拆分为包含和排除词
    pub fn new(q: &str) -> TitleQuery {
        let mut parsed = TitleQuery::default();

        for token in q.split_whitespace() {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }

            if let Some(excluded) = token.strip_prefix('-') {
                let excluded = excluded.trim();
                if !excluded.is_empty() {
                    parsed.excludes.push(excluded.to_owned());
                }
            } else {
                parsed.includes.push(token.to_owned());
            }
        }

        parsed
    }
}

/// 数据库中保存的用户组
#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "lowercase"
)]
pub enum UserGroup {
    Admin,
    User,
}

impl UserGroup {
    /// 返回用户组稳定的数据库字符串值
    pub fn as_str(&self) -> &'static str {
        match self {
            UserGroup::Admin => "admin",
            UserGroup::User => "user",
        }
    }
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

/// SeaORM 中保存的角色性别
#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "lowercase"
)]
pub enum Gender {
    Male,
    Female,
}

impl Gender {
    /// 返回角色性别稳定的数据库字符串值
    pub fn as_str(&self) -> &'static str {
        match self {
            Gender::Male => "male",
            Gender::Female => "female",
        }
    }
}
