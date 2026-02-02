// Module declarations
pub mod account;
pub mod ai_chat;
pub mod balance;
pub mod check_in;
pub mod config;
pub mod independent_key;
pub mod notification;
pub mod provider;
pub mod proxy_config;
pub mod system;
pub mod token;

// Re-export all commands for easy access
pub use account::*;
pub use ai_chat::*;
pub use balance::*;
pub use check_in::*;
pub use config::*;
pub use independent_key::*;
pub use notification::*;
pub use provider::*;
pub use proxy_config::*;
pub use system::*;
pub use token::*;
