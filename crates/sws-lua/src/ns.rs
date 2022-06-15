pub mod globals {
    //! The global namespace

    pub const SCRAP_PAGE: &str = "scrapPage"; // Function
    pub const ACCEPT_URL: &str = "acceptUrl"; // Function

    pub const SWS: &str = "sws"; // Table
}

pub mod sws {
    //! The `sws` namespace

    pub const SEED_SITEMAPS: &str = "seedSitemaps"; // Table
    pub const SEED_PAGES: &str = "seedPages"; // Table

    pub const CSV_WRITER_CONFIG: &str = "csvWriterConfig"; // Table
    pub const CRAWLER_CONFIG: &str = "crawlerConfig"; // Table

    pub mod html {
        //! The `Html` class
        pub const SELECT: &str = "select"; // Function
        pub const ROOT: &str = "root"; // Function
    }

    pub mod select {
        //! The `Select` class
        pub const ITER: &str = "iter"; // Function
        pub const ENUMERATE: &str = "enumerate"; // Function
    }

    pub mod elem_ref {
        //! The `ElemRef` class
        pub const SELECT: &str = "select"; // Function
        pub const INNER_HTML: &str = "innerHtml"; // Function
        pub const INNER_TEXT: &str = "innerText"; // Function
        pub const NAME: &str = "name"; // Function
        pub const ID: &str = "id"; // Function
        pub const HAS_CLASS: &str = "hasClass"; // Function
        pub const CLASSES: &str = "classes"; // Function
        pub const ATTR: &str = "attr"; // Function
        pub const ATTRS: &str = "attrs"; // Function
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
        pub const SEND_URL: &str = "sendUrl"; // Function
        pub const WORKER_ID: &str = "workerId"; // Function
    }
}
