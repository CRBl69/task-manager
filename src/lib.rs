#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod graphs;
mod parse_labels;
mod process_list;
mod settings;
pub use app::TaskManager;
