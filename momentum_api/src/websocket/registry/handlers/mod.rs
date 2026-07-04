//! 示例 handlers（registry trait 的最小验证）

pub mod get_connection_info;
pub mod ping;

pub use get_connection_info::GetConnectionInfoHandler;
pub use ping::PingHandler;
