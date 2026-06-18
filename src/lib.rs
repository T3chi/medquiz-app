pub mod config;
pub mod db;
pub mod domain;
pub mod llm;
pub mod models;
pub mod parsers;
pub mod services;
pub mod ui;

use std::sync::{Arc, Mutex};

use db::Database;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
}