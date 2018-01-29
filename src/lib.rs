//! # iron-archivist
//! 
//! A static file serving handler for [`iron`](https://crates.io/crates/iron).
//!
//! The core of `iron-archivist` is the [`struct Archivist`](struct.Archivist.html) which implements
//! `iron`'s [`Handler`](https://docs.rs/iron/0.6.0/iron/middleware/trait.Handler.html) trait,
//! which can be freely integerated within any application that uses `iron`.
//!

#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate url;
extern crate iron;
extern crate urlencoded;
extern crate mount;
extern crate mime_guess;
extern crate chrono;
extern crate pulldown_cmark;

mod config;
mod entry;
mod renderer;
mod archivist;

pub use config::Config;
pub use archivist::Archivist;
pub use renderer::Renderer;
pub use renderer::RenderResult;
pub use entry::Entry;
