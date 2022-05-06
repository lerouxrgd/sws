use std::path::Path;
use std::sync::mpsc::{channel, Sender};
use std::thread;

use anyhow::{anyhow, Result};
use csv::StringRecord;
use lazy_static::lazy_static;
use select::document::Document;
use select::predicate::{Attr, Class, Name, Predicate};
use serde::{Deserialize, Serialize};

use crate::scraper::{Scrapable, Sitemap};

lazy_static! {
    pub static ref URBANDICT_HEADERS: StringRecord = StringRecord::from(vec![
        "headword",
        "def_rank",
        "upvotes",
        "downvotes",
        "meaning",
        "examples",
    ]);
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UrbandictRecord {
    pub headword: String,
    pub def_rank: usize,
    pub upvotes: usize,
    pub downvotes: usize,
    pub meaning: String,
    pub examples: String,
}

pub struct Urbandict {
    #[allow(dead_code)]
    tsv_writer: thread::JoinHandle<()>,
    tx_record: Sender<UrbandictRecord>,
}

impl Urbandict {
    pub fn new<P: AsRef<Path>>(tsv_file: P) -> Result<Self> {
        if tsv_file.as_ref().exists() {
            return Err(anyhow!(
                "File already exists: {}",
                tsv_file.as_ref().display()
            ));
        }

        let mut wtr = csv::WriterBuilder::new()
            .has_headers(false)
            .delimiter(b'\t')
            .from_path(tsv_file)?;

        let (tx_record, rx_record) = channel();

        let tsv_writer = thread::spawn(move || {
            while let Ok(r) = rx_record.recv() {
                wtr.serialize(r).ok();
            }
        });

        Ok(Self {
            tsv_writer,
            tx_record,
        })
    }
}

impl Scrapable for Urbandict {
    fn site_map(&self) -> &str {
        "https://www.urbandictionary.com/sitemap-https.xml.gz"
    }

    fn accept(&self, sm: &Sitemap, url: &str) -> bool {
        match sm {
            Sitemap::Urlset => url.contains("term="),
            Sitemap::Index => true,
        }
    }

    fn parser(&self) -> Box<dyn Fn(&str) + Send> {
        let tx_record = self.tx_record.clone();

        Box::new(move |page: &str| {
            println!(">>>>>>>>>>>>> {}", page.len()); // TODO: remove

            let document = Document::from(page);

            let panels = document
                .find(Attr("id", "content").descendant(Class("def-panel")))
                .collect::<Vec<_>>();

            let mut rank = 1;

            for (i, panel) in panels.iter().enumerate() {
                let word = panel
                    .find(Class("def-header").descendant(Name("a")))
                    .next()
                    .unwrap();
                let word = word.children().next().unwrap().as_text().unwrap();

                // discard word if it contains tabs
                if word.contains("\t") {
                    return;
                }

                // skip "word of the day"
                if let Some(ribbon) = panel.find(Class("ribbon")).next() {
                    if ribbon.text().contains("Word of the Day") {
                        continue;
                    }
                }

                // current headword and definition
                let headword = word.to_string();
                if i == 0 {
                    rank = 1;
                }
                let def_rank = rank;
                rank += 1;

                // upvotes and downvotes
                let mut votes = panel.find(
                    Class("thumbs")
                        .descendant(Name("a"))
                        .descendant(Name("span")),
                );

                let upvotes = votes
                    .next()
                    .unwrap()
                    .children()
                    .next()
                    .unwrap()
                    .as_text()
                    .unwrap()
                    .parse()
                    .unwrap();

                let downvotes = votes
                    .next()
                    .unwrap()
                    .children()
                    .next()
                    .unwrap()
                    .as_text()
                    .unwrap()
                    .parse()
                    .unwrap();

                let meaning = panel
                    .find(Class("meaning"))
                    .next()
                    .unwrap()
                    .text()
                    .replace("\n", " ")
                    .replace("\t", "");

                let examples = panel
                    .find(Class("example"))
                    .next()
                    .unwrap()
                    .text()
                    .replace("\n", " ")
                    .replace("\t", "");

                let r = UrbandictRecord {
                    headword,
                    def_rank,
                    upvotes,
                    downvotes,
                    meaning,
                    examples,
                };

                tx_record.send(r).ok();
            }
        })
    }
}
