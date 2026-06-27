use redis::AsyncCommands;

#[tokio::main]
async fn main() {
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    loop {
        let mut conn = client.get_multiplexed_async_connection().await.unwrap();
        let task: String = conn.lpop("tasks", None).await.unwrap_or_default();
        if !task.is_empty() {
            println!("Processing task: {}", task);
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
