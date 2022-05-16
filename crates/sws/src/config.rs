use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwsConfig {
    sitemap_url: String,
    // TODO: unwrap or default to nb_cores -2
    num_workers: Option<usize>,
}
