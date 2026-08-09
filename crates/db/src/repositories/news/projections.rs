use crate::entities::news_sources;

use akasha_application::news::NewsSource;

impl From<news_sources::Model> for NewsSource {
    /// 将新闻来源 Entity 映射为应用层新闻来源
    fn from(value: news_sources::Model) -> Self {
        Self {
            id: value.id,
            name: value.name,
            index: value.index,
        }
    }
}
