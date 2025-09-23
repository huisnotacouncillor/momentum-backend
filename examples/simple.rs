use axum::{Router, Server};
use diesel::r2d2::{self, ConnectionManager};
use rust_backend::AppState;

#[tokio::main]
async fn main() {
    let config = rust_backend::config::Config::from_env().expect("config");
    let manager = ConnectionManager::<diesel::PgConnection>::new(config.db_url());
    let db = r2d2::Pool::builder().build(manager).unwrap();
    let redis = redis::Client::open(config.redis_url()).unwrap();
    let state = std::sync::Arc::new(rust_backend::AppState::new(db, redis, config));

    let app = Router::new()
        .route(
            "/user/:id",
            axum::routing::get(rust_backend::routes::users::get_user),
        )
        .with_state(state);

    let addr = "127.0.0.1:8000".parse().unwrap();
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
