#[cfg(feature = "tera")]
use tera::{Tera, Context};

use iron::error::IronError;
/*
#[cfg(feature = "tera")]
use iron::status;
#[cfg(feature = "tera")]
use std::error;
*/

use entry::Entry;

/// A type alias for the return type of renderer methods
pub type RenderResult = Result<String, IronError>;

/// A renderer that renders the webpage in the response
///
/// An implementation is provided for
/// [tera::Tera](https://docs.rs/tera/0.10.10/tera/struct.Tera.html).
/// Turn on the `tera` feature to use this implementation.
///
/// See `examples/simple.rs` for a minimal implementation of the renderer.
///
pub trait Renderer {
    /// Renders the list of entries in a directory.
    ///
    /// # Arguments
    /// * `path_str` - The path to the specified directory as an `str` slice
    /// * `entries`  - The entries in the specified path
    ///
    fn render_dir(&self, path_str: &str, entries: &[Entry]) -> RenderResult;
    
    /// Renders the unmodified textual content of a file.
    ///
    /// # Arguments
    /// * `path_str` - The path to the specified file as an `str` slice
    /// * `entries`  - The textual content of the file
    ///
    fn render_verbatim(&self, path_str: &str, content: &str) -> RenderResult;

    /// Renders a file as a Markdown file.
    ///
    /// # Arguments
    /// * `path_str` - The path to the specified file as an `str` slice
    /// * `content`  - The content of the file, already rendered to HTML
    ///
    fn render_markdown(&self, path_str: &str, content: &str) -> RenderResult;

    /// Renders an error message
    ///
    /// # Arguments
    /// * `path_str` - The path to the specified file as an `str` slice
    /// * `code`     - HTTP status code for the error
    /// * `message`  - An message describing the error
    ///
    fn render_error(
        &self,
        path_str: &str,
        code: usize,
        message: &str
    ) -> RenderResult;
}

