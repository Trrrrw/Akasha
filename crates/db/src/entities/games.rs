use sea_orm::{IntoActiveModel, entity::prelude::*};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "games")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub game_code: String,
    pub name_en: String,
    pub name_zh: String,
    pub index: u32,
    pub cover: String,
    pub extra: Option<String>,

    #[sea_orm(has_many)]
    pub news_items: HasMany<super::news_items::Entity>,
    #[sea_orm(has_many)]
    pub news_tags: HasMany<super::tags::Entity>,
    #[sea_orm(has_many)]
    pub characters: HasMany<super::game_characters::Entity>,
    #[sea_orm(has_many)]
    pub events: HasMany<super::game_events::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}

impl Entity {
    pub async fn create_if_not_exists(game: Model) -> Result<(), sea_orm::DbErr> {
        let conn = crate::pool();

        Self::insert(game.into_active_model())
            .on_conflict_do_nothing()
            .exec_without_returning(conn)
            .await?;

        Ok(())
    }

    pub async fn resolve_game_code(input: &str) -> Result<Option<String>, sea_orm::DbErr> {
        let input = input.trim();
        if input.is_empty() {
            return Ok(None);
        }

        let games = Self::find().all(crate::pool()).await?;
        let normalized_input = normalize_game_name(input);

        Ok(games
            .into_iter()
            .find(|game| {
                game.game_code.eq_ignore_ascii_case(input)
                    || game.name_zh == input
                    || normalize_game_name(&game.name_en) == normalized_input
            })
            .map(|game| game.game_code))
    }
}

fn normalize_game_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}
