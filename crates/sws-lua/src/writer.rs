use std::io;

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
            delimiter: default_csv_delimiter(),
            escape: None,
            flexible: false,
            terminator: default_csv_terminator(),
        }
    }
}

fn default_csv_delimiter() -> char {
    ','
}

fn default_csv_terminator() -> CsvTerminator {
    CsvTerminator::Any('\n')
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
    File(csv::Writer<fs_err::File>),
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum FileMode {
    Create,
    Append,
    Truncate,
}

impl Default for FileMode {
    fn default() -> Self {
        Self::Create
    }
}

impl From<FileMode> for fs_err::OpenOptions {
    fn from(mode: FileMode) -> Self {
        let mut opts = fs_err::OpenOptions::new();
        opts.write(true);
        match mode {
            FileMode::Create => opts.create_new(true),
            FileMode::Append => opts.create(true).append(true),
            FileMode::Truncate => opts.create(true).truncate(true),
        };
        opts
    }
}
