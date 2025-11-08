#[macro_use]
mod macros;
pub mod ping;

// Re-export handler types and their request/response types
pub use ping::PingHandler;

// Register all bidirectional message handlers
// To add a new message type, just add a line here:
//   NewMessage(NewMessageHandler),
register_handlers! {
    Ping(PingHandler),
}
