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

    pub const ITER: &str = "iter"; // Function
    pub const ENUMERATE: &str = "enumerate"; // Function

    pub mod htlm {
        //! The `Html` class
        pub const SELECT: &str = "select"; // Function
    }

    pub mod elem_ref {
        //! The `ElemRef` class
        pub const SELECT: &str = "select"; // Function
        pub const INNER_HTML: &str = "innerHtml"; // Function
        pub const INNER_TEXT: &str = "innerText"; // Function
    }

    pub const RECORD: &str = "Record"; // Table
    pub mod record {
        //! The `Record` class
        pub const NEW: &str = "new"; // Function
        pub const PUSH_FIELD: &str = "pushField"; // Function
    }

    pub mod page_location {
        //! The `PageLocation` class
        pub const KIND: &str = "kind"; // Function
        pub const GET: &str = "get"; // Function
    }

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

    pub mod context {
        //! The `SwsContext` class
        pub const PAGE_LOCATION: &str = "pageLocation"; // Function
        pub const SEND_RECORD: &str = "sendRecord"; // Function
        pub const WORKER_ID: &str = "workerId"; // Function
    }
}
