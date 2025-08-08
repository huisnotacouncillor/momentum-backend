pub mod cache;
pub mod config;
pub mod db;
pub mod middleware;
pub mod routes;
pub mod schema;
pub mod websocket;

use crate::db::DbPool;
use tracing_subscriber;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub redis: redis::Client,
}

pub fn init_tracing() {
    tracing_subscriber::fmt::init();
}
