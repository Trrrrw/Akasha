use sea_orm::{ActiveValue::Set, EntityTrait, TransactionError, TransactionTrait};

use crate::entities::{games, news_sources};

/// 列表顺序决定游戏和来源的展示顺序，资源路径遵循统一目录约定
const GAMES: &[(&str, &str, &str, &[&str])] = &[
    (
        "ys",
        "Genshin Impact",
        "原神",
        &["web_cn", "mys", "web_os_zh_tw"],
    ),
    (
        "sr",
        "Honkai: Star Rail",
        "崩坏：星穹铁道",
        &["web_cn", "mys"],
    ),
    ("zzz", "Zenless Zone Zero", "绝区零", &["web_cn", "mys"]),
    ("bh3", "Honkai Impact 3rd", "崩坏3", &["mys"]),
    ("wd", "Tears of Themis", "未定事件簿", &["mys"]),
    ("planet", "Petit Planet", "星布谷地", &["mys", "web_cn"]),
    ("hna", "Honkai: Nexus Anima", "崩坏：因缘精灵", &["mys"]),
    (
        "nodusfall",
        "Nodusfall",
        "源初之结",
        &["web_cn", "web_os_en_us"],
    ),
];

/// 写入服务运行所需的游戏和新闻来源基础数据，保留已有数据
pub(crate) async fn apply(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    db.transaction::<_, (), sea_orm::DbErr>(|txn| {
        Box::pin(async move {
            games::Entity::insert_many(GAMES.iter().enumerate().map(|(index, (id, en, zh, _))| {
                games::ActiveModel {
                    id: Set((*id).to_owned()),
                    name_en: Set((*en).to_owned()),
                    name_zh: Set((*zh).to_owned()),
                    index: Set(index as i64 + 1),
                    cover: Set(Some(format!("/assets/games/{id}/cover.avif"))),
                    icon: Set(Some(format!("/assets/games/{id}/icon.avif"))),
                }
            }))
            .on_conflict_do_nothing()
            .exec(txn)
            .await?;
            news_sources::Entity::insert_many(GAMES.iter().flat_map(|(game, _, _, sources)| {
                sources.iter().enumerate().map(move |(index, id)| {
                    let name = match *id {
                        "web_cn" => "官方网站",
                        "mys" => "米游社",
                        "web_os_zh_tw" => "国际服官网（繁体中文）",
                        "web_os_en_us" => "国际服官网（英语）",
                        _ => unreachable!("seed source must have a display name"),
                    };
                    news_sources::ActiveModel {
                        id: Set((*id).to_owned()),
                        game_id: Set((*game).to_owned()),
                        name: Set(name.to_owned()),
                        index: Set(index as i64 + 1),
                    }
                })
            }))
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
