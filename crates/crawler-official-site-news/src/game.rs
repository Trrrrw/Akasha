use std::collections::HashMap;

use crawler_core::Game;

pub trait OfficialSiteGameExt {
    const ALL: [Game; 4];
    fn categories(&self) -> Vec<&str>;
    fn extract_categories(&self, title: &str, extra_categories: Option<Vec<&str>>) -> Vec<String>;
    fn page_size(&self) -> i32;
    fn api_params(&self, page_num: i32) -> HashMap<String, String>;
    fn news_url(&self, remote_id: &i32) -> String;
    fn api_base(&self) -> &'static str;
}

impl OfficialSiteGameExt for Game {
    const ALL: [Game; 4] = [Game::Hk4e, Game::Hkrpg, Game::Nap, Game::Planet];

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
            Game::Planet => vec!["PV"],
            _ => vec![],
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

    fn page_size(&self) -> i32 {
        match self {
            Game::Hk4e => 5,
            Game::Hkrpg => 5,
            Game::Nap => 9,
            Game::Planet => 10,
            _ => 0,
        }
    }

    fn api_params(&self, page_num: i32) -> HashMap<String, String> {
        let mut params = HashMap::new();
        let page_size = self.page_size();
        let channel_id = match self {
            Game::Hk4e => "719",
            Game::Hkrpg => "255",
            Game::Nap => "273",
            Game::Planet => "1262",
            _ => "",
        };
        params.insert("iPageSize".to_string(), page_size.to_string());
        params.insert("sLangKey".to_string(), "zh-cn".to_string());
        params.insert("iChanId".to_string(), channel_id.to_string());
        match self {
            Game::Hk4e => {
                params.insert("iAppId".to_string(), "43".to_string());
            }
            Game::Hkrpg | Game::Planet => {
                params.insert("isPreview".to_string(), "0".to_string());
            }
            Game::Nap => {}
            _ => {}
        };
        params.insert("iPage".to_string(), page_num.to_string());
        params
    }

    fn news_url(&self, remote_id: &i32) -> String {
        match self {
            Game::Hk4e => format!("https://ys.mihoyo.com/main/news/detail/{remote_id}"),
            Game::Hkrpg => format!("https://sr.mihoyo.com/news/{remote_id}"),
            Game::Nap => format!("https://zzz.mihoyo.com/news/{remote_id}"),
            Game::Planet => format!("https://planet.mihoyo.com/news/detail/{remote_id}"),
            _ => String::new(),
        }
    }

    fn api_base(&self) -> &'static str {
        match self {
            Game::Hk4e => {
                "https://api-takumi-static.mihoyo.com/content_v2_user/app/16471662a82d418a/getContentList"
            }
            Game::Hkrpg => {
                "https://api-takumi-static.mihoyo.com/content_v2_user/app/1963de8dc19e461c/getContentList"
            }
            Game::Nap => {
                "https://api-takumi-static.mihoyo.com/content_v2_user/app/706fd13a87294881/getContentList"
            }
            Game::Planet => {
                "https://act-api-takumi-static.mihoyo.com/content_v2_user/app/26702175a73c4f67/getContentList"
            }
            _ => "",
        }
    }
}
