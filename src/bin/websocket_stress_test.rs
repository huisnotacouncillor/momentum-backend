use clap::{Arg, Command};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as TungsteniteMessage};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct StressTestConfig {
    websocket_url: String,
    jwt_secret: String,
    num_connections: usize,
    messages_per_connection: usize,
    connection_timeout: Duration,
    message_interval: Duration,
    max_concurrent_connections: usize,
    test_duration: Duration,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            websocket_url: "ws://127.0.0.1:8000/ws".to_string(),
            jwt_secret: "test_jwt_secret_key".to_string(),
            num_connections: 100,
            messages_per_connection: 10,
            connection_timeout: Duration::from_secs(5),
            message_interval: Duration::from_millis(100),
            max_concurrent_connections: 50,
            test_duration: Duration::from_secs(60),
        }
    }
}

#[derive(Debug, Clone)]
struct TestResults {
    total_connections_attempted: usize,
    successful_connections: usize,
    failed_connections: usize,
    total_messages_sent: usize,
    total_messages_received: usize,
    test_duration: Duration,
    average_connection_time: Duration,
    message_throughput: f64, // messages per second
    connection_success_rate: f64,
}

impl TestResults {
    fn new() -> Self {
        Self {
            total_connections_attempted: 0,
            successful_connections: 0,
            failed_connections: 0,
            total_messages_sent: 0,
            total_messages_received: 0,
            test_duration: Duration::from_secs(0),
            average_connection_time: Duration::from_secs(0),
            message_throughput: 0.0,
            connection_success_rate: 0.0,
        }
    }

    fn calculate_metrics(&mut self) {
        if self.total_connections_attempted > 0 {
            self.connection_success_rate = (self.successful_connections as f64
                / self.total_connections_attempted as f64)
                * 100.0;
        }

        if self.test_duration.as_secs() > 0 {
            self.message_throughput =
                self.total_messages_sent as f64 / self.test_duration.as_secs_f64();
        }
    }

    fn print_report(&self) {
        println!("\n=== WebSocket Stress Test Results ===");
        println!("Test Duration: {:?}", self.test_duration);
        println!(
            "Total Connections Attempted: {}",
            self.total_connections_attempted
        );
        println!("Successful Connections: {}", self.successful_connections);
        println!("Failed Connections: {}", self.failed_connections);
        println!(
            "Connection Success Rate: {:.2}%",
            self.connection_success_rate
        );
        println!(
            "Average Connection Time: {:?}",
            self.average_connection_time
        );
        println!("Total Messages Sent: {}", self.total_messages_sent);
        println!("Total Messages Received: {}", self.total_messages_received);
        println!("Message Throughput: {:.2} msg/sec", self.message_throughput);

        if self.total_messages_sent > 0 {
            let message_success_rate =
                (self.total_messages_received as f64 / self.total_messages_sent as f64) * 100.0;
            println!("Message Success Rate: {:.2}%", message_success_rate);
        }
        println!("=====================================\n");
    }
}

fn create_test_jwt(user_id: Uuid, username: &str, secret: &str) -> String {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

    #[derive(serde::Serialize)]
    struct TestClaims {
        sub: String,
        username: String,
        exp: usize,
        iat: usize,
    }

    let now = chrono::Utc::now().timestamp() as usize;
    let claims = TestClaims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp: now + 3600, // 1 hour from now
        iat: now,
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .unwrap()
}

async fn run_connection_storm_test(config: &StressTestConfig) -> TestResults {
    println!("Running Connection Storm Test...");
    println!(
        "Connections: {}, Max Concurrent: {}",
        config.num_connections, config.max_concurrent_connections
    );

    let mut results = TestResults::new();
    let start_time = Instant::now();

    let semaphore = Arc::new(Semaphore::new(config.max_concurrent_connections));
    let successful_connections = Arc::new(AtomicUsize::new(0));
    let failed_connections = Arc::new(AtomicUsize::new(0));
    let total_connection_time = Arc::new(std::sync::Mutex::new(Duration::from_secs(0)));

    let mut handles = Vec::new();

    for i in 0..config.num_connections {
        let semaphore_clone = Arc::clone(&semaphore);
        let successful_connections_clone = Arc::clone(&successful_connections);
        let failed_connections_clone = Arc::clone(&failed_connections);
        let total_connection_time_clone = Arc::clone(&total_connection_time);
        let config_clone = config.clone();

        let handle = tokio::spawn(async move {
            let _permit = semaphore_clone.acquire().await.unwrap();

            let user_id = Uuid::new_v4();
            let username = format!("storm_user_{}", i);
            let token = create_test_jwt(user_id, &username, &config_clone.jwt_secret);
            let url = format!("{}?token={}", config_clone.websocket_url, token);

            let connection_start = Instant::now();

            match Url::parse(&url) {
                Ok(parsed_url) => {
                    match timeout(config_clone.connection_timeout, connect_async(parsed_url)).await
                    {
                        Ok(Ok((ws_stream, _))) => {
                            let connection_time = connection_start.elapsed();
                            {
                                let mut total_time = total_connection_time_clone.lock().unwrap();
                                *total_time += connection_time;
                            }

                            successful_connections_clone.fetch_add(1, Ordering::Relaxed);

                            // Keep connection alive briefly
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            drop(ws_stream);
                        }
                        _ => {
                            failed_connections_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                Err(_) => {
                    failed_connections_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all connections to complete
    for handle in handles {
        let _ = handle.await;
    }

    let test_duration = start_time.elapsed();
    let successful = successful_connections.load(Ordering::Relaxed);
    let failed = failed_connections.load(Ordering::Relaxed);
    let total_time = total_connection_time.lock().unwrap().clone();

    results.total_connections_attempted = config.num_connections;
    results.successful_connections = successful;
    results.failed_connections = failed;
    results.test_duration = test_duration;
    results.average_connection_time = if successful > 0 {
        total_time / successful as u32
    } else {
        Duration::from_secs(0)
    };

    results.calculate_metrics();
    results
}

async fn run_message_throughput_test(config: &StressTestConfig) -> TestResults {
    println!("Running Message Throughput Test...");
    println!(
        "Connections: {}, Messages per Connection: {}",
        config.num_connections, config.messages_per_connection
    );

    let mut results = TestResults::new();
    let start_time = Instant::now();

    let semaphore = Arc::new(Semaphore::new(config.max_concurrent_connections));
    let successful_connections = Arc::new(AtomicUsize::new(0));
    let failed_connections = Arc::new(AtomicUsize::new(0));
    let messages_sent = Arc::new(AtomicUsize::new(0));
    let messages_received = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();

    for i in 0..config.num_connections {
        let semaphore_clone = Arc::clone(&semaphore);
        let successful_connections_clone = Arc::clone(&successful_connections);
        let failed_connections_clone = Arc::clone(&failed_connections);
        let messages_sent_clone = Arc::clone(&messages_sent);
        let messages_received_clone = Arc::clone(&messages_received);
        let config_clone = config.clone();

        let handle = tokio::spawn(async move {
            let _permit = semaphore_clone.acquire().await.unwrap();

            let user_id = Uuid::new_v4();
            let username = format!("throughput_user_{}", i);
            let token = create_test_jwt(user_id, &username, &config_clone.jwt_secret);
            let url = format!("{}?token={}", config_clone.websocket_url, token);

            match Url::parse(&url) {
                Ok(parsed_url) => {
                    match timeout(config_clone.connection_timeout, connect_async(parsed_url)).await
                    {
                        Ok(Ok((ws_stream, _))) => {
                            successful_connections_clone.fetch_add(1, Ordering::Relaxed);

                            let (mut ws_sender, mut ws_receiver) = ws_stream.split();

                            // Spawn receiver task
                            let messages_received_clone_inner =
                                Arc::clone(&messages_received_clone);
                            let receive_handle = tokio::spawn(async move {
                                while let Ok(Some(msg)) =
                                    timeout(Duration::from_secs(1), ws_receiver.next()).await
                                {
                                    if let Ok(_) = msg {
                                        messages_received_clone_inner
                                            .fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                            });

                            // Send messages
                            for msg_num in 0..config_clone.messages_per_connection {
                                let message = json!({
                                    "id": Uuid::new_v4().to_string(),
                                    "message_type": "text",
                                    "data": {
                                        "content": format!("Throughput test message {} from user {}", msg_num, i),
                                        "sequence": msg_num,
                                        "user_id": i
                                    },
                                    "timestamp": chrono::Utc::now(),
                                });

                                if ws_sender
                                    .send(TungsteniteMessage::Text(message.to_string()))
                                    .await
                                    .is_ok()
                                {
                                    messages_sent_clone.fetch_add(1, Ordering::Relaxed);
                                } else {
                                    break;
                                }

                                tokio::time::sleep(config_clone.message_interval).await;
                            }

                            // Wait a bit for responses
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            receive_handle.abort();
                        }
                        _ => {
                            failed_connections_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                Err(_) => {
                    failed_connections_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all connections to complete
    for handle in handles {
        let _ = handle.await;
    }

    let test_duration = start_time.elapsed();

    results.total_connections_attempted = config.num_connections;
    results.successful_connections = successful_connections.load(Ordering::Relaxed);
    results.failed_connections = failed_connections.load(Ordering::Relaxed);
    results.total_messages_sent = messages_sent.load(Ordering::Relaxed);
    results.total_messages_received = messages_received.load(Ordering::Relaxed);
    results.test_duration = test_duration;

    results.calculate_metrics();
    results
}

async fn run_sustained_load_test(config: &StressTestConfig) -> TestResults {
    println!("Running Sustained Load Test...");
    println!(
        "Duration: {:?}, Connections: {}",
        config.test_duration, config.num_connections
    );

    let mut results = TestResults::new();
    let start_time = Instant::now();

    let semaphore = Arc::new(Semaphore::new(config.max_concurrent_connections));
    let successful_connections = Arc::new(AtomicUsize::new(0));
    let failed_connections = Arc::new(AtomicUsize::new(0));
    let messages_sent = Arc::new(AtomicUsize::new(0));
    let messages_received = Arc::new(AtomicUsize::new(0));
    let active_connections = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();

    for i in 0..config.num_connections {
        let semaphore_clone = Arc::clone(&semaphore);
        let successful_connections_clone = Arc::clone(&successful_connections);
        let failed_connections_clone = Arc::clone(&failed_connections);
        let messages_sent_clone = Arc::clone(&messages_sent);
        let messages_received_clone = Arc::clone(&messages_received);
        let active_connections_clone = Arc::clone(&active_connections);
        let config_clone = config.clone();

        let handle = tokio::spawn(async move {
            let _permit = semaphore_clone.acquire().await.unwrap();

            let user_id = Uuid::new_v4();
            let username = format!("sustained_user_{}", i);
            let token = create_test_jwt(user_id, &username, &config_clone.jwt_secret);
            let url = format!("{}?token={}", config_clone.websocket_url, token);

            match Url::parse(&url) {
                Ok(parsed_url) => {
                    match timeout(config_clone.connection_timeout, connect_async(parsed_url)).await
                    {
                        Ok(Ok((ws_stream, _))) => {
                            successful_connections_clone.fetch_add(1, Ordering::Relaxed);
                            active_connections_clone.fetch_add(1, Ordering::Relaxed);

                            let (mut ws_sender, mut ws_receiver) = ws_stream.split();

                            // Receiver task
                            let messages_received_clone_inner =
                                Arc::clone(&messages_received_clone);
                            let receive_handle = tokio::spawn(async move {
                                while let Ok(Some(msg)) =
                                    timeout(Duration::from_secs(1), ws_receiver.next()).await
                                {
                                    if let Ok(_) = msg {
                                        messages_received_clone_inner
                                            .fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                            });

                            // Send messages periodically for the test duration
                            let test_end = start_time + config_clone.test_duration;
                            let mut message_counter = 0;

                            while Instant::now() < test_end {
                                let message = json!({
                                    "id": Uuid::new_v4().to_string(),
                                    "message_type": "text",
                                    "data": {
                                        "content": format!("Sustained test message {} from user {}", message_counter, i),
                                        "sequence": message_counter,
                                        "user_id": i,
                                        "timestamp": chrono::Utc::now()
                                    },
                                    "timestamp": chrono::Utc::now(),
                                });

                                if ws_sender
                                    .send(TungsteniteMessage::Text(message.to_string()))
                                    .await
                                    .is_ok()
                                {
                                    messages_sent_clone.fetch_add(1, Ordering::Relaxed);
                                    message_counter += 1;
                                } else {
                                    break;
                                }

                                tokio::time::sleep(config_clone.message_interval).await;
                            }

                            active_connections_clone.fetch_sub(1, Ordering::Relaxed);
                            receive_handle.abort();
                        }
                        _ => {
                            failed_connections_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                Err(_) => {
                    failed_connections_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        handles.push(handle);
    }

    // Monitor progress
    let messages_sent_monitor = Arc::clone(&messages_sent);
    let messages_received_monitor = Arc::clone(&messages_received);
    let active_connections_monitor = Arc::clone(&active_connections);
    let test_duration = config.test_duration;
    let monitor_handle = tokio::spawn(async move {
        let mut last_sent = 0;
        while start_time.elapsed() < test_duration {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let current_sent = messages_sent_monitor.load(Ordering::Relaxed);
            let current_received = messages_received_monitor.load(Ordering::Relaxed);
            let current_active = active_connections_monitor.load(Ordering::Relaxed);

            let rate = (current_sent - last_sent) as f64 / 5.0;
            println!(
                "Progress: Active Connections: {}, Messages Sent: {}, Received: {}, Rate: {:.1} msg/s",
                current_active, current_sent, current_received, rate
            );
            last_sent = current_sent;
        }
    });

    // Wait for all connections to complete
    for handle in handles {
        let _ = handle.await;
    }
    monitor_handle.abort();

    let test_duration = start_time.elapsed();

    results.total_connections_attempted = config.num_connections;
    results.successful_connections = successful_connections.load(Ordering::Relaxed);
    results.failed_connections = failed_connections.load(Ordering::Relaxed);
    results.total_messages_sent = messages_sent.load(Ordering::Relaxed);
    results.total_messages_received = messages_received.load(Ordering::Relaxed);
    results.test_duration = test_duration;

    results.calculate_metrics();
    results
}

#[tokio::main]
async fn main() {
    let matches = Command::new("WebSocket Stress Test")
        .version("1.0")
        .author("Your Name")
        .about("Stress test WebSocket server implementation")
        .arg(
            Arg::new("url")
                .short('u')
                .long("url")
                .value_name("URL")
                .help("WebSocket server URL")
                .default_value("ws://127.0.0.1:8000/ws"),
        )
        .arg(
            Arg::new("connections")
                .short('c')
                .long("connections")
                .value_name("NUMBER")
                .help("Number of concurrent connections")
                .default_value("100"),
        )
        .arg(
            Arg::new("messages")
                .short('m')
                .long("messages")
                .value_name("NUMBER")
                .help("Messages per connection")
                .default_value("10"),
        )
        .arg(
            Arg::new("duration")
                .short('d')
                .long("duration")
                .value_name("SECONDS")
                .help("Test duration in seconds")
                .default_value("60"),
        )
        .arg(
            Arg::new("max-concurrent")
                .long("max-concurrent")
                .value_name("NUMBER")
                .help("Maximum concurrent connections")
                .default_value("50"),
        )
        .arg(
            Arg::new("test-type")
                .short('t')
                .long("test-type")
                .value_name("TYPE")
                .help("Test type: storm, throughput, sustained, all")
                .default_value("all"),
        )
        .arg(
            Arg::new("message-interval")
                .long("message-interval")
                .value_name("MILLISECONDS")
                .help("Interval between messages in milliseconds")
                .default_value("100"),
        )
        .get_matches();

    let mut config = StressTestConfig::default();
    config.websocket_url = matches.get_one::<String>("url").unwrap().clone();
    config.num_connections = matches
        .get_one::<String>("connections")
        .unwrap()
        .parse()
        .unwrap();
    config.messages_per_connection = matches
        .get_one::<String>("messages")
        .unwrap()
        .parse()
        .unwrap();
    config.test_duration = Duration::from_secs(
        matches
            .get_one::<String>("duration")
            .unwrap()
            .parse()
            .unwrap(),
    );
    config.max_concurrent_connections = matches
        .get_one::<String>("max-concurrent")
        .unwrap()
        .parse()
        .unwrap();
    config.message_interval = Duration::from_millis(
        matches
            .get_one::<String>("message-interval")
            .unwrap()
            .parse()
            .unwrap(),
    );

    let test_type = matches.get_one::<String>("test-type").unwrap();

    println!("=== WebSocket Stress Test Tool ===");
    println!("Server URL: {}", config.websocket_url);
    println!("Test Configuration:");
    println!("  Connections: {}", config.num_connections);
    println!(
        "  Messages per Connection: {}",
        config.messages_per_connection
    );
    println!("  Test Duration: {:?}", config.test_duration);
    println!("  Max Concurrent: {}", config.max_concurrent_connections);
    println!("  Message Interval: {:?}", config.message_interval);
    println!("==================================\n");

    match test_type.as_str() {
        "storm" => {
            let results = run_connection_storm_test(&config).await;
            results.print_report();
        }
        "throughput" => {
            let results = run_message_throughput_test(&config).await;
            results.print_report();
        }
        "sustained" => {
            let results = run_sustained_load_test(&config).await;
            results.print_report();
        }
        "all" => {
            println!("Running all stress tests...\n");

            let storm_results = run_connection_storm_test(&config).await;
            storm_results.print_report();

            tokio::time::sleep(Duration::from_secs(2)).await;

            let throughput_results = run_message_throughput_test(&config).await;
            throughput_results.print_report();

            tokio::time::sleep(Duration::from_secs(2)).await;

            let sustained_results = run_sustained_load_test(&config).await;
            sustained_results.print_report();

            println!("=== Summary of All Tests ===");
            println!(
                "Connection Storm - Success Rate: {:.2}%",
                storm_results.connection_success_rate
            );
            println!(
                "Message Throughput - Rate: {:.2} msg/sec",
                throughput_results.message_throughput
            );
            println!(
                "Sustained Load - Rate: {:.2} msg/sec",
                sustained_results.message_throughput
            );
            println!("============================");
        }
        _ => {
            eprintln!("Invalid test type. Use: storm, throughput, sustained, or all");
            std::process::exit(1);
        }
    }
}
