pub mod globals {
    //! The global namespace

    pub const ACCEPT_URL: &str = "acceptUrl"; // Function
    pub const SCRAP_PAGE: &str = "scrapPage"; // Function

    pub const SWS: &str = "sws"; // Table
}

pub mod sws {
    //! The `sws` namespace

    pub const SITEMAP_URL: &str = "sitemapUrl"; // String
    pub const CSV_WRITER_CONFIG: &str = "csvWriterConfig"; // Table
    pub const CRAWLER_CONFIG: &str = "crawlerConfig"; // Table

    pub const SELECT_ITER: &str = "selectIter"; // Function
    pub const NEW_RECORD: &str = "newRecord"; // Function

    pub const LOCATION: &str = "Location"; // Table
    pub mod location {
        //! The `Location` enum
        pub const URL: &str = "URL"; // String
        pub const PATH: &str = "PATH"; // String
    }

    pub const SITEMAP: &str = "Sitemap"; // Table
    pub mod sitemap {
        //! The `Sitemap` enum
        pub const INDEX: &str = "INDEX"; // String
        pub const URL_SET: &str = "URL_SET"; // String
    }
}
