use sea_orm::{
    ActiveValue::Set, ConnectOptions, ConnectionTrait, Database, DatabaseConnection, EntityTrait,
    FromQueryResult, IntoActiveModel, PaginatorTrait, TransactionError, TransactionTrait,
};
use uuid::Uuid;

use crate::{
    Db, DbError, DbOptions,
    entities::{
        audit_logs, characters, game_events, games, news, news_sources, news_tags, news_tags_link,
        oauth_accounts, user_api_keys, user_groups, user_refresh_tokens, users, worker_states,
    },
};

const IMPORT_BATCH_SIZE: u64 = 50;

/// PostgreSQL 备份导入后的统计信息
#[derive(Debug, Default)]
pub struct ImportSummary {
    /// 成功导入的表数量
    pub table_count: usize,
    /// 成功导入的记录数量
    pub row_count: u64,
}

impl ImportSummary {
    fn add_table(&mut self, row_count: u64) {
        self.table_count += 1;
        self.row_count += row_count;
    }
}

macro_rules! copy_entity {
    ($source:expr, $target:expr, $entity:ty) => {{
        let mut page = 0;
        let mut row_count = 0;

        loop {
            let rows = <$entity>::find()
                .order_by_id_asc()
                .paginate($source, IMPORT_BATCH_SIZE)
                .fetch_page(page)
                .await?;
            if rows.is_empty() {
                break;
            }

            row_count += rows.len() as u64;
            <$entity>::insert_many(rows.into_iter().map(|model| model.into_active_model()))
                .exec($target)
                .await?;
            page += 1;
        }

        row_count
    }};
}

/// 从临时 PostgreSQL 数据库分页导入 SQLite
pub async fn import_postgres_to_sqlite(
    postgres_url: &str,
    sqlite_path: String,
) -> Result<ImportSummary, DbError> {
    let mut connect_options = ConnectOptions::new(postgres_url.to_owned());
    connect_options
        .max_connections(2)
        .min_connections(1)
        .sqlx_logging(false);
    let source = Database::connect(connect_options)
        .await
        .map_err(DbError::Import)?;

    let target = Db::init(DbOptions { sqlite_path }).await?;
    target.import_from_postgres(&source).await
}

impl Db {
    async fn import_from_postgres(
        &self,
        source: &DatabaseConnection,
    ) -> Result<ImportSummary, DbError> {
        let source = source.clone();
        let summary = self
            .conn()
            .transaction::<_, ImportSummary, sea_orm::DbErr>(|txn| {
                Box::pin(async move {
                    clear_target(txn).await?;

                    let mut summary = ImportSummary::default();
                    summary.add_table(copy_entity!(&source, txn, games::Entity));
                    summary.add_table(copy_entity!(&source, txn, news_sources::Entity));
                    summary.add_table(copy_entity!(&source, txn, news_tags::Entity));
                    summary.add_table(copy_entity!(&source, txn, news::Entity));
                    summary.add_table(copy_entity!(&source, txn, news_tags_link::Entity));
                    summary.add_table(copy_entity!(&source, txn, characters::Entity));
                    summary.add_table(copy_game_events(&source, txn).await?);
                    summary.add_table(copy_entity!(&source, txn, users::Entity));
                    summary.add_table(copy_entity!(&source, txn, oauth_accounts::Entity));
                    summary.add_table(copy_entity!(&source, txn, user_groups::Entity));
                    summary.add_table(copy_user_api_keys(&source, txn).await?);
                    summary.add_table(copy_entity!(&source, txn, user_refresh_tokens::Entity));
                    summary.add_table(copy_entity!(&source, txn, worker_states::Entity));
                    summary.add_table(copy_entity!(&source, txn, audit_logs::Entity));

                    Ok(summary)
                })
            })
            .await
            .map_err(map_transaction_error)?;

        // 导入后再次执行清洗和必需种子，覆盖旧数据库中的遗留值
        self.normalize_legacy_news_covers().await?;
        self.seed_required_data().await?;

        Ok(summary)
    }
}

async fn clear_target<C>(db: &C) -> Result<(), sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    news_tags_link::Entity::delete_many().exec(db).await?;
    news::Entity::delete_many().exec(db).await?;
    news_tags::Entity::delete_many().exec(db).await?;
    news_sources::Entity::delete_many().exec(db).await?;
    characters::Entity::delete_many().exec(db).await?;
    game_events::Entity::delete_many().exec(db).await?;
    worker_states::Entity::delete_many().exec(db).await?;
    user_refresh_tokens::Entity::delete_many().exec(db).await?;
    user_api_keys::Entity::delete_many().exec(db).await?;
    user_groups::Entity::delete_many().exec(db).await?;
    oauth_accounts::Entity::delete_many().exec(db).await?;
    audit_logs::Entity::delete_many().exec(db).await?;
    users::Entity::delete_many().exec(db).await?;
    games::Entity::delete_many().exec(db).await?;
    Ok(())
}

#[derive(Debug, FromQueryResult)]
struct SourceGameEvent {
    game_id: String,
    id: String,
    title: String,
    introduction: Option<String>,
    main_text: Option<String>,
    start: Option<chrono::DateTime<chrono::FixedOffset>>,
    end: Option<chrono::DateTime<chrono::FixedOffset>>,
    tags: Option<Vec<String>>,
    url: Option<String>,
}

async fn copy_game_events<C>(source: &DatabaseConnection, target: &C) -> Result<u64, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    let mut page = 0;
    let mut row_count = 0;

    loop {
        let rows = game_events::Entity::find()
            .order_by_id_asc()
            .into_model::<SourceGameEvent>()
            .paginate(source, IMPORT_BATCH_SIZE)
            .fetch_page(page)
            .await?;
        if rows.is_empty() {
            break;
        }

        row_count += rows.len() as u64;
        game_events::Entity::insert_many(rows.into_iter().map(|row| game_events::ActiveModel {
            game_id: Set(row.game_id),
            id: Set(row.id),
            title: Set(row.title),
            introduction: Set(row.introduction),
            main_text: Set(row.main_text),
            start: Set(row.start),
            end: Set(row.end),
            tags: Set(row.tags.map(|tags| serde_json::json!(tags))),
            url: Set(row.url),
        }))
        .exec(target)
        .await?;
        page += 1;
    }

    Ok(row_count)
}

#[derive(Debug, FromQueryResult)]
struct SourceUserApiKey {
    id: Uuid,
    user_id: Uuid,
    name: String,
    key_prefix: String,
    key_hash: String,
    scopes: Vec<String>,
    expires_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    last_used_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    revoked_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    created_at: chrono::DateTime<chrono::FixedOffset>,
}

async fn copy_user_api_keys<C>(
    source: &DatabaseConnection,
    target: &C,
) -> Result<u64, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    let mut page = 0;
    let mut row_count = 0;

    loop {
        let rows = user_api_keys::Entity::find()
            .order_by_id_asc()
            .into_model::<SourceUserApiKey>()
            .paginate(source, IMPORT_BATCH_SIZE)
            .fetch_page(page)
            .await?;
        if rows.is_empty() {
            break;
        }

        row_count += rows.len() as u64;
        user_api_keys::Entity::insert_many(rows.into_iter().map(|row| {
            user_api_keys::ActiveModel {
                id: Set(row.id),
                user_id: Set(row.user_id),
                name: Set(row.name),
                key_prefix: Set(row.key_prefix),
                key_hash: Set(row.key_hash),
                scopes: Set(serde_json::json!(row.scopes)),
                expires_at: Set(row.expires_at),
                last_used_at: Set(row.last_used_at),
                revoked_at: Set(row.revoked_at),
                created_at: Set(row.created_at),
            }
        }))
        .exec(target)
        .await?;
        page += 1;
    }

    Ok(row_count)
}

fn map_transaction_error(error: TransactionError<sea_orm::DbErr>) -> DbError {
    match error {
        TransactionError::Connection(error) | TransactionError::Transaction(error) => {
            DbError::Import(error)
        }
    }
}
