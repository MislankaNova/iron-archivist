use chrono::{DateTime, Utc};

use std::fs::DirEntry;
use std::io;

/// Directory entry used for rendering
///
/// The `struct Entry` can by converted from Rust's standard `DirEntry`. It contains only the data
/// needed for the purpose of rendering an directory index.
///
#[derive(Debug, Clone)]
pub struct Entry {
    pub is_dir: bool,
    pub file_name: String,
    pub modified: String,
}

impl Entry {
    pub fn from(e: &DirEntry) -> io::Result<Self> {
        let md = e.metadata()?;
        Ok(Entry {
            is_dir: md.is_dir(),
            file_name: String::from(
                e.file_name()
                 .into_string()
                 .map_err(|_| io::Error::new(
                     io::ErrorKind::Other,
                     "File name is not valid UTF-8."
                 ))?),
            modified: DateTime::<Utc>::from(md.modified()?)
                .format("%Y-%m-%d %R").to_string(),
        })
    }
}
