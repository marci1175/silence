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

#[cfg(feature = "udp")]
pub mod udp;

#[cfg(feature = "voice")]
pub mod voice;

#[cfg(feature = "video")]
pub mod video;

#[doc(hidden)]
#[cfg(test)]
pub mod tests;