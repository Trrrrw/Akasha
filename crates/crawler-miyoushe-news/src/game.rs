use std::collections::HashMap;

use crawler_core::Game;

pub trait MiyousheGameExt {
    const ALL: [Game; 7];
    const API_BASE: &'static str;
    fn code(&self) -> &'static str;
    fn gid(&self) -> &'static str;
    fn api_params(
        &self,
        last_id: &u32,
        page_size: &u32,
        news_type: &i32,
    ) -> HashMap<String, String>;
    fn post_api_url(&self, remote_id: &str) -> String;
    fn news_url(&self, remote_id: &i32) -> String;
    fn categories(&self) -> Vec<&str>;
    fn extract_categories(&self, title: &str, extra_categories: Option<Vec<&str>>) -> Vec<String>;
}

impl MiyousheGameExt for Game {
    const ALL: [Game; 7] = [
        Game::Hk4e,
        Game::Hkrpg,
        Game::Nap,
        Game::Bh3,
        Game::Wd,
        Game::Planet,
        Game::Hna,
    ];

    const API_BASE: &'static str = "https://bbs-api.miyoushe.com/painter/wapi/getNewsList";

    fn code(&self) -> &'static str {
        match self {
            Game::Hk4e => "ys",
            Game::Hkrpg => "sr",
            Game::Nap => "zzz",
            Game::Bh3 => "bh3",
            Game::Wd => "wd",
            Game::Planet => "planet",
            Game::Hna => "hna",
        }
    }

    fn gid(&self) -> &'static str {
        match self {
            Game::Hk4e => "2",
            Game::Hkrpg => "6",
            Game::Nap => "8",
            Game::Bh3 => "1",
            Game::Wd => "4",
            Game::Planet => "10",
            Game::Hna => "9",
        }
    }

    fn api_params(
        &self,
        last_id: &u32,
        page_size: &u32,
        news_type: &i32,
    ) -> HashMap<String, String> {
        let mut params = HashMap::new();
        params.insert("gids".to_string(), self.gid().to_string());
        params.insert("last_id".to_string(), last_id.to_string());
        params.insert("page_size".to_string(), page_size.to_string());
        params.insert("type".to_string(), news_type.to_string());
        params
    }

    fn post_api_url(&self, remote_id: &str) -> String {
        format!(
            "https://bbs-api.miyoushe.com/post/wapi/getPostFull?gids={}&post_id={remote_id}&read=1",
            self.gid()
        )
    }

    fn news_url(&self, remote_id: &i32) -> String {
        format!(
            "https://www.miyoushe.com/{}/article/{remote_id}",
            self.code()
        )
    }

    fn categories(&self) -> Vec<&str> {
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
            Game::Wd => vec!["活动PV", "剧情PV", "角色相关"],
            Game::Planet => vec!["PV"],
            Game::Hna => vec![],
        }
    }

    fn extract_categories(&self, title: &str, extra_categories: Option<Vec<&str>>) -> Vec<String> {
        let mut categories: Vec<String> = extra_categories
            .unwrap_or_default()
            .into_iter()
            .map(str::to_string)
            .collect();

        for pre_categories in self.categories() {
            if title.contains(pre_categories) {
                categories.push(pre_categories.to_string());
            }
        }
        if categories.is_empty() {
            categories.push("其他".to_string());
        }

        categories
    }
}
