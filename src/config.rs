use toml;

use iron::mime::{TopLevel, Mime};
use mime_guess::get_mime_type;

use std::collections::BTreeSet;
use std::io;
use std::fs::File;
use std::io::prelude::*;
use std::path::Component;
use std::path::Path;
use std::ffi::OsStr;
use std::ffi::OsString;

/// How a file should be served to the user.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AccessMethod {
    /// Render the file using Markdown
    Markdown,
    /// Return the textual content without modification
    Verbatim,
    /// Return the raw file
    Raw,
    /// Show the directory
    Dir,
}

impl AccessMethod {
    pub fn is_file(&self) -> bool {
        match self {
            &AccessMethod::Dir => false,
            _ => true,
        }
    }

    pub fn is_dir(&self) -> bool {
        match self {
            &AccessMethod::Dir => true,
            _ => false,
        }
    }
}

/// The server configuration
///
/// The configuration can be parsed from a TOML file. An example of such a configuration file is
/// shown below:
///
/// ```toml
/// # The directory to serve
/// # This one, for example, would serve the `src' directory
/// root_dir = "src"
/// 
/// # The address and port to listen
/// listen = "localhost:5000"
/// 
/// # If allow_all is on then all files in the served directory are served
/// # otherwise only files whose extensions are on the `allow' list are served
/// allow_all = false
/// 
/// # Only files with these extensions are allowed
/// allow = [ "rs", "txt", "md", "html", "css", "jpg", "png" ]
/// 
/// # Files with these extensions will be rendered as Markdown script
/// markdown = [ "md" ]
/// ```
///
#[derive(Debug, Clone)]
pub struct Config {
    /// The path to the directory containing the served files
    pub root_dir: String,
    /// The address and port that the server listens to
    pub listen: String,
    /// Whether or not files with extensions not `allow'ed should be served
    pub allow_all: bool,
    /// The set of file extensions that will be allowed to be served
    pub allowed_extensions: BTreeSet<OsString>,
    /// The set of file names that will be allowed to be served
    pub allowed_file_names: BTreeSet<OsString>,
    /// The set of file names that will be blocked from access
    pub blocked_file_names: BTreeSet<OsString>,
    /// The set of file extensions that will be treated as Markdown files
    pub markdown: BTreeSet<OsString>,
}

impl Config {
    /// Loads the configuration from a TOML file.
    ///
    /// # Arguments
    /// * `ext` - A reference to the file extension as an `OsStr`
    ///
    /// # Error
    /// Returns an error if the file cannot be loaded, or if it is malformed.
    ///
    pub fn load<P: AsRef<Path>>(path: &P) -> io::Result<Self> {
        let mut file = File::open(path.as_ref())?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        toml::from_str::<RawConfig>(content.as_str())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
            .map(Config::from)
    }

    /// Returns the access method specified for the file at the specified path
    /// Returns None if the file is not allowed
    ///
    /// # Arguments
    /// * `path` - The path to the specified file
    ///
    /// # Error
    /// Returns an error if the metadata of the file cannot be accessed.
    ///
    pub fn method_for<P: AsRef<Path>>(&self, path: &P)
            -> io::Result<Option<AccessMethod>> {
        // If metadata cannot be accessed then do not allow
        let path = path.as_ref();
        let metadata = path.metadata()?;

        // If the file name cannot be extracted then do not allow
        let file_name = match path.file_name() {
            Some(s) => s,
            None => OsStr::new(""),
        };

        // If the file name begins with a `.', and the file is not allowed
        // then do not allow
        // Note that these files are not allowed even if `allow-all' is set
        for c in path.components() {
            match c {
                Component::Normal(s) => {
                    let s_str = s.to_str().unwrap_or("");
                    if (s.len() > 0
                            && s_str.starts_with(".")
                            && !self.allowed_file_names.contains(s))
                            || self.blocked_file_names.contains(s) {
                        return Ok(None);
                    }
                },

                // Make sure that the path does not go up
                //
                // This is in fact not actually needed
                // iron operates above the hyper library and accepts two types of URIs from hyper requests
                // the first being AbsoluteUri, which is parsed using crate url's Url parser
                //   which is known to handle `..' correctly
                // the second being AbsolutePath, which is handled by iron when creating an iron Request
                //     from a hyper Request
                //   url's Url parser is used later again, so `..' should be handled
                //
                // In theory the request path should not contain any `..' at all
                //   but just in case, this is checked.
                Component::ParentDir => return Ok(None),

                _ => (),
            }
        }

        // If the path leads to a directory then access as directory
        if metadata.is_dir() {
            return Ok(Some(AccessMethod::Dir));
        }

        // If we cannot get the extension, and the file is not explicitly allowed
        // then do not allow
        // Unless allow-all is set
        let ext = match path.extension() {
            Some(ext) => ext,
            None => {
                if self.allowed_file_names.contains(file_name)
                        || self.allow_all {
                    // If the file name is allowed but it does not contain an extension
                    // Then treat the file as plain text
                    return Ok(Some(AccessMethod::Verbatim));
                } else {
                    return Ok(None);
                }
            },
        };

        // If the extension is not allowed, and the file name is not allowed
        // then do not allow
        if !self.allow_all
                && !self.allowed_extensions.contains(ext)
                && !self.allowed_file_names.contains(file_name) {
            return Ok(None);
        }

        // If the extension should be treated as markdown then do so
        if self.markdown.contains(ext) {
            return Ok(Some(AccessMethod::Markdown));
        }

        // Otherwise guess the Mime of the file
        // If the file is text then access its textual content
        // Otherwise access the raw file
        let ext_str = match ext.to_str() {
            Some(s) => s,
            None => return Ok(None),
        };
        let mime = get_mime_type(ext_str);
        match mime {
            Mime(TopLevel::Text, _, _) => Ok(Some(AccessMethod::Verbatim)),
            _ => Ok(Some(AccessMethod::Raw)),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config::from(RawConfig::default())
    }
}

impl From<RawConfig> for Config {
    fn from(raw: RawConfig) -> Self {
        Config {
            root_dir:           raw.root_dir,
            listen:             raw.listen,
            allow_all:          raw.allow_all,
            allowed_extensions:
                raw.allowed_extensions.unwrap_or(BTreeSet::new())
                   .iter()
                   .map(OsString::from)
                   .collect(),
            allowed_file_names:
                raw.allowed_file_names.unwrap_or(BTreeSet::new())
                   .iter()
                   .map(OsString::from)
                   .collect(),
            blocked_file_names:
                raw.blocked_file_names.unwrap_or(BTreeSet::new())
                   .iter()
                   .map(OsString::from)
                   .collect(),
            markdown:
                raw.markdown.unwrap_or({
                    let mut set = BTreeSet::new();
                    set.insert(String::from("md"));
                    set
                }).iter()
                  .map(OsString::from)
                  .collect(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct RawConfig {
    pub root_dir: String,
    pub listen: String,
    pub allow_all: bool,
    pub allowed_extensions: Option<BTreeSet<String>>,
    pub allowed_file_names: Option<BTreeSet<String>>,
    pub blocked_file_names: Option<BTreeSet<String>>,
    pub markdown: Option<BTreeSet<String>>,
}

impl Default for RawConfig {
    fn default() -> Self {
        RawConfig {
            root_dir: String::from("."),
            listen: String::from("localhost:5000"),
            allow_all: false,
            allowed_extensions: None,
            allowed_file_names: None,
            blocked_file_names: None,
            markdown: Some({
                let mut set = BTreeSet::new();
                set.insert(String::from("md"));
                set
            }),
        }
    }
}
