pub mod enums;
pub mod models;

use diesel::PgConnection;
use diesel::r2d2::{self, ConnectionManager as DbConnectionManager};

pub type DbPool = r2d2::Pool<DbConnectionManager<PgConnection>>;
