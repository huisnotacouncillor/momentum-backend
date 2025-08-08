use rust_backend::cache::redis::{get_cache, set_cache};

#[tokio::test]
async fn test_cache() {
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    set_cache(&client, "test", &"value", 10).await;
    let value: Option<String> = get_cache(&client, "test").await;
    assert_eq!(value, Some("value".to_string()));
}
