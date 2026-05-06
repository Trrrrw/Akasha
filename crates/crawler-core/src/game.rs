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
    #[inline]
    pub const fn name_zh(&self) -> &'static str {
        match self {
            Game::Hk4e => "原神",
            Game::Hkrpg => "崩坏：星穹铁道",
            Game::Nap => "绝区零",
            Game::Bh3 => "崩坏3",
            Game::Wd => "未定事件簿",
            Game::Planet => "星布谷地",
            Game::Hna => "崩坏：因缘精灵",
        }
    }

    #[inline]
    pub const fn name_en(&self) -> &'static str {
        match self {
            Game::Hk4e => "Genshin Impact",
            Game::Hkrpg => "Honkai: Star Rail",
            Game::Nap => "Zenless Zone Zero",
            Game::Bh3 => "Honkai Impact 3",
            Game::Wd => "Tears of Themis",
            Game::Planet => "Petit Planet",
            Game::Hna => "Honkai: Nexus Anima",
        }
    }

    #[inline]
    pub const fn index(&self) -> usize {
        match self {
            Game::Hk4e => 0,
            Game::Hkrpg => 1,
            Game::Nap => 2,
            Game::Bh3 => 3,
            Game::Wd => 4,
            Game::Planet => 5,
            Game::Hna => 6,
        }
    }

    #[inline]
    pub const fn default_cover(&self) -> &'static str {
        match self {
            Game::Hk4e => "https://ys.mihoyo.com/main/_nuxt/img/holder.37207c1.jpg",
            Game::Hkrpg => {
                "https://webstatic.mihoyo.com/upload/op-public/2023/01/24/b74ae5e3a8e8b021b67ea26e27a215f2_184072581688764639.png"
            }
            Game::Nap => {
                "https://webstatic.mihoyo.com/upload/op-public/2022/09/17/a425b5ccb44c72e342cf3a6e488dc445_771169193410538499.jpg"
            }
            Game::Bh3 => "http://static.event.mihoyo.com/bh3_homepage/images/pic/picture/01.jpg",
            Game::Wd => {
                "https://webstatic.mihoyo.com/upload/wd-wiki/2022/02/28/127729/120b5d4273290a9425ccb280becbf6c3_4216098022458368688.png"
            }
            Game::Planet => "https://planet.mihoyo.com/_nuxt/img/poster.d17fa7b.png",
            Game::Hna => "http://static.event.mihoyo.com/bh3_homepage/images/pic/picture/01.jpg",
        }
    }
}
