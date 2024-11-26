//!  This feature provides functions and abstractions for sending both Voice and Video packets.

#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "server")]
pub mod server;

/// Custom networking (udp) errors.
#[derive(thiserror::Error, Debug)]
pub enum UdpError {
    /// This error is thrown when a message has failed to send.
    #[error("Failed to send message.")]
    SendError(std::io::Error),

    /// This error is thrown when the [`UdpSocket`] has failed to bind to the local address.
    #[error("Failed to bind to local address.")]
    BindError(std::io::Error), 

    /// This error is thrown when no remote address could be resolved.
    #[error("Failed to resolve remote address.")]
    ConnectionError(std::io::Error),
}

/// Defines the Result enum with the [`UdpError`] error type.
pub type Result<T> = ::std::result::Result<T, UdpError>;