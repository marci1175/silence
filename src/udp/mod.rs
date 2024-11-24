//!  This feature provides functions and abstractions for sending both Voice and Video packets.

#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "server")]
pub mod server;
