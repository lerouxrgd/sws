use sws_crawler::{crawl_site, CrawlerConfig, CrawlingContext, Scrapable, ScrapingContext, Seed};

#[tokio::test]
#[should_panic(
    expected = "Invalid seed config, cannot use Seed::RobotsTxt when `crawler_conf.robot` is defined"
)]
async fn validate_robot_config() {
    struct DummyScraper;

    impl Scrapable for DummyScraper {
        type Config = ();
        fn new(_config: &Self::Config) -> anyhow::Result<Self>
        where
            Self: Sized,
        {
            Ok(Self)
        }
        fn seed(&self) -> Seed {
            sws_crawler::Seed::RobotsTxt("https://dummy-url.com/robots.txt".into())
        }
        fn accept(&self, _url: &str, _ctx: CrawlingContext) -> bool {
            true
        }
        fn scrap(&mut self, _page: String, _ctx: ScrapingContext) -> anyhow::Result<()> {
            Ok(())
        }
    }

    let scraper_conf = ();
    let crawler_conf = CrawlerConfig {
        robot: Some("https://dummy-url.com/robots.txt".into()),
        ..Default::default()
    };

    crawl_site::<DummyScraper>(&crawler_conf, &scraper_conf)
        .await
        .unwrap();
}
