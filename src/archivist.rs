use url::percent_encoding::percent_decode;

use iron::prelude::*;
use iron::status;
use iron::Url;
use iron::headers::ContentType;
use iron::middleware::Handler;
use iron::modifiers::Header;
use iron::modifiers::Redirect;
use mount;
use url;
use urlencoded::UrlEncodedQuery;

use pulldown_cmark::{html, Parser};

use std::cmp::Ordering;
use std::fs;
use std::fs::*;
use std::io;
use std::io::prelude::*;
use std::path::*;
use std::sync::Arc;

use config::*;
use entry::*;
use renderer::*;

/// Order in which the entries should be sorted
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
enum EntryOrder {
    Lexicographical,
    Chronological,
}

/// A handler that serves static directory indices and files
///
/// `Archivist` implements `iron`'s [`Handler`](https://docs.rs/iron/0.6.0/iron/middleware/trait.Handler.html) trait,
/// making it possible to incorporate `Archivist` into any other `iron` application.
///
pub struct Archivist<T: Renderer> {
    raw: bool,
    root: PathBuf,
    config: Config,
    renderer: Arc<T>,
}

impl<T> Archivist<T> where T: Renderer {
    /// Summons a `Archivist` using a certain configuration.
    ///
    /// # Arguments
    /// * `config`   - The configuration to be used
    /// * `renderer` - A shared, thread safe pointer to the renderer.
    ///
    pub fn summon(config: &Config, renderer: Arc<T>) -> Archivist<T> {
        Archivist {
            raw: false,
            root: PathBuf::from(Path::new(&config.root_dir)),
            config: config.clone(),
            renderer: renderer
        }
    }

    /// Summons a `Archivist` which serves all files as-is, using a certain configuration.
    ///
    /// # Arguments
    /// * `config` - The configuration to be used
    /// * `renderer` - A shared, thread safe pointer to the renderer.
    ///
    pub fn summon_raw(config: &Config, renderer: Arc<T>) -> Archivist<T> {
        Archivist {
            raw: true,
            root: PathBuf::from(Path::new(&config.root_dir)),
            config: config.clone(),
            renderer: renderer
        }
    }

    #[inline]
    fn not_found(&self, path_str: &str) -> IronResult<Response> {
        self.renderer.render_error(
            &path_str,
            404,
            "The requested archive is not found"
        ).map(|s| Response::with((
            s,
            Header(ContentType::html()),
            status::NotFound
        )))
    }

    #[inline]
    fn invalid_format(&self, path_str: &str) -> IronResult<Response> {
        self.renderer.render_error(
            &path_str,
            416,
            "The requested file is not valid UTF8"
        ).map(|s| Response::with((
            s,
            Header(ContentType::html()),
            status::NotFound
        )))
    }
}

impl<T> Handler for Archivist<T> where T: Renderer + Send + Sync + 'static {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {

        // Construct the path to the file being accessed
        let mut path = PathBuf::new();
        for n in req.url.path() {
            // The path in the url is percent encoded
            // So it needs to be decoded here
            path.push(String::from(percent_decode(n.as_bytes())
                                   .decode_utf8()
                                   .unwrap()
            ));
        }
        let path_string = format!("{}", path.as_path().display());

        // If we are at the root and that the archivist is mounted (using iron/mount)
        // Then make sure that there is a trailing slash
        //
        // First check if the Archivist is mounted
        if let Some(url) = req.extensions.get::<mount::OriginalUrl>() {
            // Then check if the last part of the original url
            // differs from the one we are now handling
            //
            // If there is a difference
            // it indicates that a trailing slash have been added by the mount
            if url.path().last() != req.url.path().last() {
                let mut original_url : url::Url = url.clone().into();
                // Redirect to the proper url (with trailing slash)
                original_url.path_segments_mut().unwrap().push("");
                return Ok(Response::with((
                    "Redirecting to root directory.",
                    Redirect(Url::from_generic_url(original_url).unwrap()),
                    status::MovedPermanently
                )));
            }
        }

        // Construct the path to the actual file in the file system
        let full_path = self.root.as_path().join(&path);

        let access = match self.config.method_for(&full_path) {
            Ok(Some(m)) => m,
            _ => return self.not_found(&path_string),
        };

        // Does the path have a trailing slash?
        let trailing_slash = match req.url.path().last() {
            Some(&"") => true,
            _ => false,
        };

        // Directories must have the trailing slash
        // Files must not have the trailing slash
        if trailing_slash && access.is_file()
                || !trailing_slash && access.is_dir() {
            return self.not_found(&path_string);
        }
       
        // If serving raw AND the path leads to a file
        // Then serve the file directly
        // Otherwise return error 404
        if self.raw {
            return if access.is_file() {
                serve_raw(&full_path)
            } else {
                return self.not_found(&path_string);
            }
        }

        match access {
            AccessMethod::Markdown => {
                // Serve the file rendered as Markdown
                let mut file = match File::open(&full_path) {
                    Ok(f) => f,
                    Err(_) => return self.not_found(&path_string),
                };
                let mut content = String::new();
                // If the file is UTF-8
                // Then render the content of the file
                // And render it as Markdown script
                if let Ok(_) = file.read_to_string(&mut content) {
                    let parser = Parser::new(&content);
                    let mut result = String::new();
                    html::push_html(&mut result, parser);
                    self.renderer.render_markdown(&path_string, &result)
                        .map(response_html)
                // Otherwise there is an error
                } else {
                    self.invalid_format(&path_string)
                }
            },

            AccessMethod::Verbatim => {
                // Serve the unmodified text context of the file
                let mut file = match File::open(&full_path) {
                    Ok(f) => f,
                    Err(_) => return self.not_found(&path_string),
                };
                let mut content = String::new();
                // If the file is UTF-8
                // Then return the file as it is
                if let Ok(_) = file.read_to_string(&mut content) {
                    self.renderer.render_verbatim(&path_string, &content)
                        .map(response_html)
                // Otherwise there is an error
                } else {
                    self.invalid_format(&path_string)
                }
            },

            AccessMethod::Raw => {
                serve_raw(&full_path)
            },

            AccessMethod::Dir => {
                // First collect the directory entries that we can access
                let mut dir_entries : Vec<DirEntry> = fs::read_dir(full_path)
                    .unwrap()
                    .flat_map(|e| e)
                    .filter(|e: &DirEntry| {
                        self.config.method_for(&e.path())
                            .unwrap_or(None)
                            .is_some()
                    } )
                    .collect();

                // Then sort the entries in the order specified
                match get_entry_order(req) {
                    Some(EntryOrder::Lexicographical) =>
                        dir_entries.sort_by(cmp_entry_by_name),

                    Some(EntryOrder::Chronological) => 
                        dir_entries.sort_by(cmp_entry_by_modified),

                    None => (),
                }

                // Then collect them as entry objects
                let entries : Vec<Entry> = dir_entries.iter()
                    .map(|de| Entry::from(de).unwrap())
                    .collect();

                // Render the page, generate an HTTP response
                self.renderer.render_dir(&path_string, &entries)
                    .map(response_html)
            },
        }
    }
}

// Wrap the rendered page in a response body
fn response_html(content: String) -> Response {
    Response::with((
        content.as_str(),
        status::Ok,
        Header(ContentType::html())
    ))
}

// Stock response bodies
#[inline]
fn serve_raw<P: AsRef<Path>>(full_path: &P) -> IronResult<Response> {
    Ok(Response::with((full_path.as_ref(),
                       status::Ok)))
}

#[inline]
fn get_entry_order(req: &mut Request) -> Option<EntryOrder> {
    if let Ok(ref queries) = req.get_ref::<UrlEncodedQuery>() {
        queries.get("order")
            .and_then(|v| v.first())
            .and_then(|o| match o.as_str() {
                "lexicographical" => Some(EntryOrder::Lexicographical),
                "chronological" => Some(EntryOrder::Chronological),
                _ => None
            } )
    } else {
        None
    }
}

// Comparers for DirEntry
fn cmp_entry_by_name(e1: &DirEntry, e2: &DirEntry) -> Ordering {
    // TODO: implement naturalistic comparison of strings
    e1.file_name().cmp(&e2.file_name())
}

fn cmp_entry_by_modified(e1: &DirEntry, e2: &DirEntry) -> Ordering {
    try_cmp_entry_by_modified(e1, e2).unwrap_or(Ordering::Equal)
}

fn try_cmp_entry_by_modified(e1: &DirEntry, e2: &DirEntry)
        -> Result<Ordering, io::Error> {
    let e1_meta = e1.metadata()?;
    let e1_modified = e1_meta.modified()?;

    let e2_meta = e2.metadata()?;
    let e2_modified = e2_meta.modified()?;

    Ok(e1_modified.cmp(&e2_modified))
}

