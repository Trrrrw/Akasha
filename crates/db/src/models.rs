use sea_orm::entity::prelude::*;

#[derive(Debug, Clone, Copy, Default)]
pub struct NewsStats {
    pub total: u64,
    pub video: u64,
    pub article: u64,
    pub latest_publish_time: Option<DateTimeWithTimeZone>,
    pub latest_video_publish_time: Option<DateTimeWithTimeZone>,
}

#[derive(Debug, Default)]
pub struct TitleQuery {
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
}

impl TitleQuery {
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
    pub fn as_str(&self) -> &'static str {
        match self {
            UserGroup::Admin => "admin",
            UserGroup::User => "user",
        }
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
pub enum WorkerPhase {
    #[sea_orm(string_value = "initial_backfill")]
    InitialBackfill,

    #[sea_orm(string_value = "incremental")]
    Incremental,
}

impl WorkerPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InitialBackfill => "initial_backfill",
            Self::Incremental => "incremental",
        }
    }
}

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
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Failed => "failed",
        }
    }
}

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
    pub fn as_str(&self) -> &'static str {
        match self {
            Gender::Male => "male",
            Gender::Female => "female",
        }
    }
}
