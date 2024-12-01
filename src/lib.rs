#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub
)]

//!
//! # Silence
//! Silence. _**Break the silence**_.
//!
//! A crate for creating voip services the easiest and the most reliable way.
//!
//! The crate provides a few important things:
//! * Client abstractions: For receiving and sending packets of data (Voice, and Video data).
//! * Server abstractions: For relaying the incoming information to all of the clients.
//!
//! *Promises reliability and efficiency by using [tokio](https://crates.io/crates/tokio) and [parking_lot](https://crates.io/crates/parking_lot).*
//!
//! ***The crate uses [UDP](https://en.wikipedia.org/wiki/User_Datagram_Protocol) for it's real time communication, which does not mitigate against packet loss.***
//!

/// Maximum Transmission Unit size.
/// This is a limit of the packet length the client can send, so that messages wont get fragmented.
pub const MTU_MAX_PACKET_SIZE: usize = 1300;

#[cfg(feature = "udp")]
pub mod udp;

#[cfg(any(feature = "voice", feature = "video"))]
pub mod packet;

#[doc(hidden)]
#[cfg(test)]
pub mod tests;

/// Re-export all of the functionalities depending on the features this crate has enabled.
pub use silence_core;