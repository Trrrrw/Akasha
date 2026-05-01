// 使用默认的 Game
// use crawler_core::Game;

// pub trait DemoGameExt {
//     const ALL: [Game; 1];
// }

// impl DemoGameExt for Game {
//     const ALL: [Game; 1] = [Game::Hk4e];
// }

// 自定义新 Game
#[derive(Debug)]
pub enum Game {
    NewGame1,
}

impl Game {
    pub const ALL: [Game; 1] = [Game::NewGame1];
    // other fn
}
