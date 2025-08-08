use clap::{Arg, Command};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::io::{self, Write};
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as TungsteniteMessage};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct ClientConfig {
    server_url: String,
    jwt_token: String,
    user_id: Uuid,
    username: String,
    email: String,
    auto_ping: bool,
    ping_interval: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_url: "ws://127.0.0.1:8000/ws".to_string(),
            jwt_token: String::new(),
            user_id: Uuid::new_v4(),
            username: "test_user".to_string(),
            email: "test@example.com".to_string(),
            auto_ping: true,
            ping_interval: Duration::from_secs(30),
        }
    }
}

/// Create a test JWT token
fn create_test_jwt(user_id: Uuid, username: &str, email: &str, secret: &str) -> String {
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

    #[derive(serde::Serialize)]
    struct TestClaims {
        sub: Uuid,
        email: String,
        username: String,
        exp: u64,
        iat: u64,
        jti: String,
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let claims = TestClaims {
        sub: user_id,
        email: email.to_string(),
        username: username.to_string(),
        exp: now + 3600, // 1 hour from now
        iat: now,
        jti: Uuid::new_v4().to_string(),
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .unwrap()
}

async fn connect_websocket(
    config: &ClientConfig,
) -> Result<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    Box<dyn std::error::Error>,
> {
    let url_with_token = format!("{}?token={}", config.server_url, config.jwt_token);
    let url = Url::parse(&url_with_token)?;

    println!("Connecting to: {}", config.server_url);
    println!("User: {} ({})", config.username, config.user_id);

    let (ws_stream, response) = connect_async(url).await?;

    println!("WebSocket connection established!");
    println!("Response status: {}", response.status());
    println!("Response headers:");
    for (name, value) in response.headers() {
        println!("  {}: {:?}", name, value);
    }
    println!();

    Ok(ws_stream)
}

async fn handle_interactive_mode(config: ClientConfig) -> Result<(), Box<dyn std::error::Error>> {
    let ws_stream = connect_websocket(&config).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Spawn a task to handle incoming messages
    let receive_handle = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(TungsteniteMessage::Text(text)) => {
                    match serde_json::from_str::<serde_json::Value>(&text) {
                        Ok(json_msg) => {
                            let msg_type = json_msg
                                .get("message_type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");

                            let timestamp = json_msg
                                .get("timestamp")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");

                            println!("\n📨 [{} | {}] Received message:", msg_type, timestamp);

                            match msg_type {
                                "text" => {
                                    if let Some(content) = json_msg
                                        .get("data")
                                        .and_then(|d| d.get("content"))
                                        .and_then(|c| c.as_str())
                                    {
                                        println!("   💬 Text: {}", content);
                                    }
                                }
                                "notification" => {
                                    println!(
                                        "   🔔 Notification: {}",
                                        json_msg.get("data").unwrap_or(&json!({}))
                                    );
                                }
                                "system_message" => {
                                    println!(
                                        "   🖥️  System: {}",
                                        json_msg.get("data").unwrap_or(&json!({}))
                                    );
                                }
                                "user_joined" => {
                                    if let Some(username) = json_msg
                                        .get("data")
                                        .and_then(|d| d.get("username"))
                                        .and_then(|u| u.as_str())
                                    {
                                        println!("   👋 {} joined the chat", username);
                                    }
                                }
                                "user_left" => {
                                    if let Some(username) = json_msg
                                        .get("data")
                                        .and_then(|d| d.get("username"))
                                        .and_then(|u| u.as_str())
                                    {
                                        println!("   👋 {} left the chat", username);
                                    }
                                }
                                "pong" => {
                                    println!("   🏓 Pong received");
                                }
                                _ => {
                                    println!("   📄 Raw: {}", text);
                                }
                            }
                            println!(
                                "   🕒 Full message: {}",
                                serde_json::to_string_pretty(&json_msg).unwrap_or(text)
                            );
                        }
                        Err(_) => {
                            println!("\n📨 Raw text message: {}", text);
                        }
                    }
                }
                Ok(TungsteniteMessage::Close(close_frame)) => {
                    println!("\n🔐 Connection closed: {:?}", close_frame);
                    break;
                }
                Ok(TungsteniteMessage::Ping(data)) => {
                    println!("\n🏓 Ping received: {:?}", data);
                }
                Ok(TungsteniteMessage::Pong(data)) => {
                    println!("\n🏓 Pong received: {:?}", data);
                }
                Err(e) => {
                    println!("\n❌ WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Auto-ping task using channel communication
    let (ping_tx, mut ping_rx) = tokio::sync::mpsc::unbounded_channel();
    let auto_ping = config.auto_ping;
    let ping_interval_duration = config.ping_interval;

    let ping_handle = tokio::spawn(async move {
        if !auto_ping {
            return;
        }

        let mut ping_interval = tokio::time::interval(ping_interval_duration);
        loop {
            ping_interval.tick().await;

            let ping_message = json!({
                "id": Uuid::new_v4().to_string(),
                "message_type": "ping",
                "data": {
                    "timestamp": chrono::Utc::now()
                },
                "timestamp": chrono::Utc::now(),
            });

            if ping_tx.send(ping_message.to_string()).is_err() {
                break;
            } else {
                println!("🏓 Sent automatic ping");
            }
        }
    });

    // Interactive command loop
    println!("\n🎮 Interactive WebSocket Client");
    println!("Available commands:");
    println!("  text <message>     - Send text message");
    println!("  ping               - Send ping");
    println!("  notification <msg> - Send notification");
    println!("  system <msg>       - Send system message");
    println!("  json <json>        - Send raw JSON message");
    println!("  help               - Show this help");
    println!("  quit               - Quit client");
    println!();

    let stdin = tokio::io::stdin();
    let mut lines = tokio::io::BufReader::new(stdin).lines();

    loop {
        print!("websocket> ");
        io::stdout().flush().unwrap();

        tokio::select! {
            // Handle auto-ping messages
            ping_msg = ping_rx.recv() => {
                if let Some(msg) = ping_msg {
                    if let Err(e) = ws_sender.send(TungsteniteMessage::Text(msg)).await {
                        println!("❌ Failed to send auto-ping: {}", e);
                        break;
                    }
                }
            },
            line = lines.next_line() => {
                match line {
                    Ok(Some(input)) => {
                        let input = input.trim();
                        if input.is_empty() {
                            continue;
                        }

                        let parts: Vec<&str> = input.splitn(2, ' ').collect();
                        let command = parts[0];
                        let args = if parts.len() > 1 { parts[1] } else { "" };

                        match command {
                            "quit" | "exit" | "q" => {
                                println!("👋 Goodbye!");
                                break;
                            },
                            "help" | "h" => {
                                println!("Available commands:");
                                println!("  text <message>     - Send text message");
                                println!("  ping               - Send ping");
                                println!("  notification <msg> - Send notification");
                                println!("  system <msg>       - Send system message");
                                println!("  json <json>        - Send raw JSON message");
                                println!("  help               - Show this help");
                                println!("  quit               - Quit client");
                            },
                            "text" => {
                                if args.is_empty() {
                                    println!("❌ Usage: text <message>");
                                    continue;
                                }

                                let message = json!({
                                    "id": Uuid::new_v4().to_string(),
                                    "message_type": "text",
                                    "data": {
                                        "content": args
                                    },
                                    "timestamp": chrono::Utc::now(),
                                });

                                if let Err(e) = ws_sender.send(TungsteniteMessage::Text(message.to_string())).await {
                                    println!("❌ Failed to send message: {}", e);
                                } else {
                                    println!("✅ Sent text message: {}", args);
                                }
                            },
                            "ping" => {
                                let message = json!({
                                    "id": Uuid::new_v4().to_string(),
                                    "message_type": "ping",
                                    "data": {
                                        "manual": true,
                                        "timestamp": chrono::Utc::now()
                                    },
                                    "timestamp": chrono::Utc::now(),
                                });

                                if let Err(e) = ws_sender.send(TungsteniteMessage::Text(message.to_string())).await {
                                    println!("❌ Failed to send ping: {}", e);
                                } else {
                                    println!("✅ Sent ping");
                                }
                            },
                            "notification" => {
                                if args.is_empty() {
                                    println!("❌ Usage: notification <message>");
                                    continue;
                                }

                                let message = json!({
                                    "id": Uuid::new_v4().to_string(),
                                    "message_type": "notification",
                                    "data": {
                                        "content": args,
                                        "priority": "normal"
                                    },
                                    "timestamp": chrono::Utc::now(),
                                });

                                if let Err(e) = ws_sender.send(TungsteniteMessage::Text(message.to_string())).await {
                                    println!("❌ Failed to send notification: {}", e);
                                } else {
                                    println!("✅ Sent notification: {}", args);
                                }
                            },
                            "system" => {
                                if args.is_empty() {
                                    println!("❌ Usage: system <message>");
                                    continue;
                                }

                                let message = json!({
                                    "id": Uuid::new_v4().to_string(),
                                    "message_type": "system_message",
                                    "data": {
                                        "content": args,
                                        "source": "client"
                                    },
                                    "timestamp": chrono::Utc::now(),
                                });

                                if let Err(e) = ws_sender.send(TungsteniteMessage::Text(message.to_string())).await {
                                    println!("❌ Failed to send system message: {}", e);
                                } else {
                                    println!("✅ Sent system message: {}", args);
                                }
                            },
                            "json" => {
                                if args.is_empty() {
                                    println!("❌ Usage: json <json_string>");
                                    continue;
                                }

                                match serde_json::from_str::<serde_json::Value>(args) {
                                    Ok(_) => {
                                        if let Err(e) = ws_sender.send(TungsteniteMessage::Text(args.to_string())).await {
                                            println!("❌ Failed to send JSON: {}", e);
                                        } else {
                                            println!("✅ Sent raw JSON");
                                        }
                                    },
                                    Err(e) => {
                                        println!("❌ Invalid JSON: {}", e);
                                    }
                                }
                            },
                            _ => {
                                println!("❌ Unknown command: {}. Type 'help' for available commands.", command);
                            }
                        }
                    },
                    Ok(None) => break,
                    Err(e) => {
                        println!("❌ Error reading input: {}", e);
                        break;
                    }
                }
            },
            // Check if receive task is finished (non-blocking)
            _ = tokio::time::sleep(Duration::from_millis(1)) => {
                if receive_handle.is_finished() {
                    println!("📡 Receive task ended");
                    break;
                }
            }
        }
    }

    // Clean shutdown
    ping_handle.abort();
    let _ = ws_sender.send(TungsteniteMessage::Close(None)).await;
    let _ = timeout(Duration::from_secs(1), receive_handle).await;

    Ok(())
}

async fn run_benchmark_mode(
    config: ClientConfig,
    num_messages: usize,
    interval_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Running benchmark mode:");
    println!("  Messages: {}", num_messages);
    println!("  Interval: {}ms", interval_ms);
    println!();

    let ws_stream = connect_websocket(&config).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let start_time = std::time::Instant::now();
    let mut messages_sent = 0;
    let mut messages_received = 0;

    // Spawn receiver task
    let receive_handle = tokio::spawn(async move {
        let mut count = 0;
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(TungsteniteMessage::Text(_)) => {
                    count += 1;
                    if count % 10 == 0 {
                        println!("📈 Received {} messages", count);
                    }
                }
                Ok(TungsteniteMessage::Close(_)) => break,
                Err(e) => {
                    println!("❌ Receive error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        count
    });

    // Send messages
    let interval = Duration::from_millis(interval_ms);
    for i in 0..num_messages {
        let message = json!({
            "id": Uuid::new_v4().to_string(),
            "message_type": "text",
            "data": {
                "content": format!("Benchmark message #{}", i + 1),
                "sequence": i + 1,
                "timestamp": chrono::Utc::now()
            },
            "timestamp": chrono::Utc::now(),
        });

        match ws_sender
            .send(TungsteniteMessage::Text(message.to_string()))
            .await
        {
            Ok(_) => {
                messages_sent += 1;
                if (i + 1) % 10 == 0 {
                    println!("📤 Sent {} messages", i + 1);
                }
            }
            Err(e) => {
                println!("❌ Failed to send message #{}: {}", i + 1, e);
                break;
            }
        }

        if interval_ms > 0 {
            tokio::time::sleep(interval).await;
        }
    }

    // Wait for remaining messages
    println!("⏳ Waiting for remaining messages...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let _ = ws_sender.send(TungsteniteMessage::Close(None)).await;

    if let Ok(received_count) = timeout(Duration::from_secs(3), receive_handle).await {
        messages_received = received_count.unwrap_or(0);
    }

    let duration = start_time.elapsed();

    println!("\n📊 Benchmark Results:");
    println!("  Duration: {:?}", duration);
    println!("  Messages sent: {}", messages_sent);
    println!("  Messages received: {}", messages_received);
    println!(
        "  Send rate: {:.2} msg/sec",
        messages_sent as f64 / duration.as_secs_f64()
    );
    if messages_received > 0 {
        println!(
            "  Receive rate: {:.2} msg/sec",
            messages_received as f64 / duration.as_secs_f64()
        );
        println!(
            "  Round-trip success: {:.1}%",
            (messages_received as f64 / messages_sent as f64) * 100.0
        );
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("WebSocket Client")
        .version("1.0")
        .author("Your Name")
        .about("WebSocket client for testing Momentum backend")
        .arg(
            Arg::new("url")
                .short('u')
                .long("url")
                .value_name("URL")
                .help("WebSocket server URL")
                .default_value("ws://127.0.0.1:8000/ws"),
        )
        .arg(
            Arg::new("token")
                .short('t')
                .long("token")
                .value_name("TOKEN")
                .help("JWT token for authentication"),
        )
        .arg(
            Arg::new("jwt-secret")
                .long("jwt-secret")
                .value_name("SECRET")
                .help("JWT secret for generating test tokens")
                .default_value("your-secret-key"),
        )
        .arg(
            Arg::new("username")
                .long("username")
                .value_name("USERNAME")
                .help("Username for test token")
                .default_value("test_user"),
        )
        .arg(
            Arg::new("email")
                .long("email")
                .value_name("EMAIL")
                .help("Email for test token")
                .default_value("test@example.com"),
        )
        .arg(
            Arg::new("benchmark")
                .short('b')
                .long("benchmark")
                .help("Run in benchmark mode")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("messages")
                .short('m')
                .long("messages")
                .value_name("COUNT")
                .help("Number of messages to send in benchmark mode")
                .default_value("100"),
        )
        .arg(
            Arg::new("interval")
                .short('i')
                .long("interval")
                .value_name("MILLISECONDS")
                .help("Interval between messages in benchmark mode")
                .default_value("100"),
        )
        .arg(
            Arg::new("no-auto-ping")
                .long("no-auto-ping")
                .help("Disable automatic ping messages")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let mut config = ClientConfig::default();
    config.server_url = matches.get_one::<String>("url").unwrap().clone();
    config.username = matches.get_one::<String>("username").unwrap().clone();
    config.email = matches.get_one::<String>("email").unwrap().clone();
    config.auto_ping = !matches.get_flag("no-auto-ping");

    // Generate or use provided JWT token
    config.jwt_token = if let Some(token) = matches.get_one::<String>("token") {
        token.clone()
    } else {
        let jwt_secret = matches.get_one::<String>("jwt-secret").unwrap();
        create_test_jwt(config.user_id, &config.username, &config.email, jwt_secret)
    };

    println!("🔌 WebSocket Client Starting");
    println!("Configuration:");
    println!("  Server URL: {}", config.server_url);
    println!("  Username: {}", config.username);
    println!("  Email: {}", config.email);
    println!("  User ID: {}", config.user_id);
    println!("  Auto-ping: {}", config.auto_ping);
    println!(
        "  Token: {}...{}",
        &config.jwt_token[..std::cmp::min(20, config.jwt_token.len())],
        if config.jwt_token.len() > 40 {
            &config.jwt_token[config.jwt_token.len() - 20..]
        } else {
            ""
        }
    );
    println!();

    if matches.get_flag("benchmark") {
        let num_messages: usize = matches.get_one::<String>("messages").unwrap().parse()?;
        let interval: u64 = matches.get_one::<String>("interval").unwrap().parse()?;
        run_benchmark_mode(config, num_messages, interval).await
    } else {
        handle_interactive_mode(config).await
    }
}
