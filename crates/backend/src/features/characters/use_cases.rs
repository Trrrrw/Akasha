use akasha_db::{
    Db, DbError,
    repositories::characters::{self, CharListFilter, CharSummary},
};

pub(super) async fn list(
    db: &Db,
    filter: CharListFilter,
) -> Result<(u64, Vec<CharSummary>), DbError> {
    characters::get_char_list(db, filter).await
}
