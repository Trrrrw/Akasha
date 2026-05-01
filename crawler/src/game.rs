use std::collections::HashMap;

use strum::{Display, EnumString};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum Game {
    Bh3,
    Hk4e,
    Wd,
    Hkrpg,
    Nap,
    Planet,
    Hna,
}

impl Game {
    pub fn news_categories(&self) -> Vec<&str> {
        match self {
            Game::Hk4e => vec![
                "角色演示",
                "拾枝杂谈",
                "角色PV",
                "EP",
                "MV",
                "前瞻特别节目",
                "版本PV",
                "过场动画",
                "剧情PV",
                "角色预告",
                "PV",
                "幕后",
                "动画短片",
                "提瓦特美食札记",
                "活动汇总",
                "衣装PV",
                "流光拾遗之旅",
                "蒙德茶会",
                "璃月雅集",
                "寻味之旅",
                "角色逸闻",
                "CM短片",
                "景区联动纪录片",
                "风物集短片",
                "PV短片",
            ],
            Game::Hkrpg => vec![
                "走近星穹",
                "角色PV",
                "千星纪游",
                "版本PV",
                "PV",
                "动画短片",
                "EP",
                "MV",
                "OP",
                "黄金史诗PV",
                "剧情PV",
                "遥远星球之歌",
                "前瞻特别节目",
                "星穹研习会",
            ],
            Game::Nap => vec![
                "角色展示",
                "角色PV",
                "EP",
                "MV",
                "PV",
                "过场动画",
                "战斗设计幕后",
                "代理人战斗情报",
                "前瞻特别节目",
                "版本PV",
                "日常影像",
                "幕间PV",
                "动画短片",
                "策划面对面",
            ],
            Game::Bh3 => vec![
                "游戏PV",
                "动画短片",
                "角色PV",
                "视频集锦",
                "服装视频",
                "主题曲/音乐",
                "过场动画",
            ],
            Game::Wd => vec!["活动PV", "剧情PV", "角色相关", "其他"],
            Game::Planet => vec!["PV"],
            Game::Hna => vec![],
        }
    }
}
