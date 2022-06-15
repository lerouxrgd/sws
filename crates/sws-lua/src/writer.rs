use std::{fs, io};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CsvWriterConfig {
    #[serde(default = "default_csv_delimiter")]
    pub delimiter: char,
    #[serde(default)]
    pub escape: Option<char>,
    #[serde(default)]
    pub flexible: bool,
    #[serde(default = "default_csv_terminator")]
    pub terminator: CsvTerminator,
}

impl Default for CsvWriterConfig {
    fn default() -> Self {
        Self {
            delimiter: ',',
            escape: None,
            flexible: false,
            terminator: CsvTerminator::Any('\n'),
        }
    }
}

fn default_csv_delimiter() -> char {
    CsvWriterConfig::default().delimiter
}

fn default_csv_terminator() -> CsvTerminator {
    CsvWriterConfig::default().terminator
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CsvTerminator {
    CRLF,
    Any(char),
}

impl From<CsvTerminator> for csv::Terminator {
    fn from(source: CsvTerminator) -> Self {
        match source {
            CsvTerminator::CRLF => Self::CRLF,
            CsvTerminator::Any(c) => Self::Any(c as u8),
        }
    }
}

impl From<&CsvWriterConfig> for csv::WriterBuilder {
    fn from(c: &CsvWriterConfig) -> Self {
        let mut builder = csv::WriterBuilder::new();
        builder.delimiter(c.delimiter as u8);
        builder.terminator(c.terminator.into());
        builder.flexible(c.flexible);
        if let Some(escape) = c.escape {
            builder.double_quote(false);
            builder.escape(escape as u8);
        } else {
            builder.double_quote(true);
        }
        builder
    }
}

pub enum CsvWriter {
    File(csv::Writer<fs::File>),
    Stdout(csv::Writer<io::Stdout>),
}

impl CsvWriter {
    pub fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::File(wtr) => wtr.flush(),
            Self::Stdout(wtr) => wtr.flush(),
        }
    }

    pub fn write_record<I, T>(&mut self, record: I) -> csv::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<[u8]>,
    {
        match self {
            Self::File(wtr) => wtr.write_record(record),
            Self::Stdout(wtr) => wtr.write_record(record),
        }
    }
}
