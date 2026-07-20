use sea_orm::{ActiveValue::Set, EntityTrait, TransactionError, TransactionTrait};

use crate::entities::{games, news_sources};

const GAME_SEEDS: &[GameSeed] = &[
    GameSeed {
        id: "ys",
        name_en: "Genshin Impact",
        name_zh: "原神",
        index: 1,
        cover: Some("/assets/games/ys/cover.avif"),
        icon: Some("/assets/games/ys/icon.avif"),
    },
    GameSeed {
        id: "sr",
        name_en: "Honkai: Star Rail",
        name_zh: "崩坏：星穹铁道",
        index: 2,
        cover: Some("/assets/games/sr/cover.avif"),
        icon: Some("/assets/games/sr/icon.avif"),
    },
    GameSeed {
        id: "zzz",
        name_en: "Zenless Zone Zero",
        name_zh: "绝区零",
        index: 3,
        cover: Some("/assets/games/zzz/cover.avif"),
        icon: Some("/assets/games/zzz/icon.avif"),
    },
    GameSeed {
        id: "bh3",
        name_en: "Honkai Impact 3rd",
        name_zh: "崩坏3",
        index: 4,
        cover: Some("/assets/games/bh3/cover.avif"),
        icon: Some("/assets/games/bh3/icon.avif"),
    },
    GameSeed {
        id: "wd",
        name_en: "Tears of Themis",
        name_zh: "未定事件簿",
        index: 5,
        cover: Some("/assets/games/wd/cover.avif"),
        icon: Some("/assets/games/wd/icon.avif"),
    },
    GameSeed {
        id: "planet",
        name_en: "Petit Planet",
        name_zh: "星布谷地",
        index: 6,
        cover: Some("/assets/games/planet/cover.avif"),
        icon: Some("/assets/games/planet/icon.avif"),
    },
    GameSeed {
        id: "hna",
        name_en: "Honkai: Nexus Anima",
        name_zh: "崩坏：因缘精灵",
        index: 7,
        cover: Some("/assets/games/hna/cover.avif"),
        icon: Some("/assets/games/hna/icon.avif"),
    },
];

const NEWS_SOURCE_SEEDS: &[NewsSourceSeed] = &[
    NewsSourceSeed {
        id: "web_cn",
        game_id: "ys",
        name: "官方网站",
        index: 1,
    },
    NewsSourceSeed {
        id: "mys",
        game_id: "ys",
        name: "米游社",
        index: 2,
    },
    NewsSourceSeed {
        id: "web_cn",
        game_id: "sr",
        name: "官方网站",
        index: 1,
    },
    NewsSourceSeed {
        id: "mys",
        game_id: "sr",
        name: "米游社",
        index: 2,
    },
    NewsSourceSeed {
        id: "web_cn",
        game_id: "zzz",
        name: "官方网站",
        index: 1,
    },
    NewsSourceSeed {
        id: "mys",
        game_id: "zzz",
        name: "米游社",
        index: 2,
    },
    NewsSourceSeed {
        id: "web_cn",
        game_id: "planet",
        name: "官方网站",
        index: 1,
    },
    NewsSourceSeed {
        id: "mys",
        game_id: "planet",
        name: "米游社",
        index: 2,
    },
    NewsSourceSeed {
        id: "web_cn",
        game_id: "hna",
        name: "官方网站",
        index: 1,
    },
    NewsSourceSeed {
        id: "mys",
        game_id: "hna",
        name: "米游社",
        index: 2,
    },
    NewsSourceSeed {
        id: "mys",
        game_id: "bh3",
        name: "米游社",
        index: 1,
    },
    NewsSourceSeed {
        id: "mys",
        game_id: "wd",
        name: "米游社",
        index: 1,
    },
];

pub(crate) async fn apply(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    db.transaction::<_, (), sea_orm::DbErr>(|txn| {
        Box::pin(async move {
            games::Entity::insert_many(GAME_SEEDS.iter().map(GameSeed::active_model))
                .on_conflict_do_nothing()
                .exec(txn)
                .await?;

            news_sources::Entity::insert_many(
                NEWS_SOURCE_SEEDS.iter().map(NewsSourceSeed::active_model),
            )
            .on_conflict_do_nothing()
            .exec(txn)
            .await?;

            Ok(())
        })
    })
    .await
    .map_err(|error| match error {
        TransactionError::Connection(error) | TransactionError::Transaction(error) => error,
    })
}

struct GameSeed {
    id: &'static str,
    name_en: &'static str,
    name_zh: &'static str,
    index: i64,
    cover: Option<&'static str>,
    icon: Option<&'static str>,
}

impl GameSeed {
    fn active_model(&self) -> games::ActiveModel {
        games::ActiveModel {
            id: Set(self.id.to_owned()),
            name_en: Set(self.name_en.to_owned()),
            name_zh: Set(self.name_zh.to_owned()),
            index: Set(self.index),
            cover: Set(self.cover.map(str::to_owned)),
            icon: Set(self.icon.map(str::to_owned)),
        }
    }
}

struct NewsSourceSeed {
    id: &'static str,
    game_id: &'static str,
    name: &'static str,
    index: i64,
}

impl NewsSourceSeed {
    fn active_model(&self) -> news_sources::ActiveModel {
        news_sources::ActiveModel {
            id: Set(self.id.to_owned()),
            game_id: Set(self.game_id.to_owned()),
            name: Set(self.name.to_owned()),
            index: Set(self.index),
        }
    }
}
