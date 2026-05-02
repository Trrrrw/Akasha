use std::sync::Arc;

use crawler_core::CrawlerTask;
use crawler_miyoushe_news::NewsMiyousheTask;
use crawler_official_site_news::NewsOfficialSiteTask;

pub fn get() -> Vec<Arc<dyn CrawlerTask + Send + Sync>> {
    vec![Arc::new(NewsOfficialSiteTask), Arc::new(NewsMiyousheTask)]
}
