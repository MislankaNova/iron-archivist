//! A simple server with a very simple renderer for demonstration
//!
//! Run `cargo run --example simple' in the package directory,
//! and visit localhost:5000 to browse files in thie package.

extern crate iron;
extern crate iron_archivist;

use iron::prelude::*;

use std::ffi::OsString;
use std::sync::Arc;

use iron_archivist::*;

/// A very simple renderer implementation. Does not render really beautiful pages.
struct SimpleRenderer;

impl Renderer for SimpleRenderer {
    fn render_dir(&self, path_str: &str, entries: &[Entry])
            -> RenderResult {
        let mut result = format!(
            "<h1>/{}</h1><ul><li><a href=\"..\">..</a>",
            path_str
        );
        for e in entries.iter() {
            result.push_str(&format!(
                "<li><a href=\"{}{}\">{}</a></li>",
                &e.file_name,
                // A path to a directory must end with an ending slash
                if e.is_dir { "/" } else { "" },
                &e.file_name,
            ));
        }
        result.push_str("</ul>");
        Ok(result)
    }

    fn render_verbatim(&self, path_str: &str, content: &str) -> RenderResult {
        Ok(format!(
            "<h1>{}</h1><a href=\".\">Back</a><pre>{}</pre>",
            path_str,
            content
        ))
    }

    fn render_markdown(&self, path_str: &str, content: &str) -> RenderResult {
        Ok(format!(
            "<h1>{}</h1><a href=\".\">Back</a>{}",
            path_str,
            content
        ))
    }

    fn render_error(
            &self,
            _: &str,
            code: usize,
            message: &str) -> RenderResult {
        Ok(format!(
            "<h1>Error {}</h1><p>{}</p>",
            code,
            message
        ))
    }
}

fn main() {
    // Use the default configuration
    // And allow things that we would expect to see
    // in a Rust library
    let mut config = Config::default();
    config.allowed_extensions.insert(OsString::from("rs"));
    config.allowed_extensions.insert(OsString::from("md"));
    config.allowed_extensions.insert(OsString::from("css"));
    config.allowed_extensions.insert(OsString::from("html"));
    config.allowed_file_names.insert(OsString::from(".gitignore"));
    config.allowed_file_names.insert(OsString::from("Cargo.toml"));
    config.blocked_file_names.insert(OsString::from("target"));
    // Use our very simple renderer
    let renderer = Arc::new(SimpleRenderer);

    // Summon our archivist
    let archivist = Archivist::summon(&config, renderer);
    // Archivist is an Iron Handler
    // Which can be used to create an Iron server
    Iron::new(archivist)
        .http(&config.listen)
        .unwrap();
    // The server now listens localhost:5000
}
