use dotenv::dotenv;

pub struct Config {
    pub db_url: String,
    pub redis_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv().ok();
        Config {
            db_url: std::env::var("DATABASE_URL").expect("DATABASE_URL is not set"),
            redis_url: std::env::var("REDIS_URL").expect("REDIS_URL is not set"),
        }
    }
}
