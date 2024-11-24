//! Provides functions and helpers for the client side of the Voip service.
use tokio::net::{ToSocketAddrs, UdpSocket};
use uuid::Uuid;

/// Client struct definition, mnade to simplify the usage of a client.
#[derive(Debug)]
pub struct Client {
    /// The unique identificator for this [`Client`] instance.
    uuid: Uuid,

    /// The [`UdpSocket`] bound to the remote address specified when creating the instance.
    udp_socket: UdpSocket,
}

impl Client {
    /// Creates a new Client instance, automaticly sets up the [`UdpSocket`].
    pub async fn new<T: ToSocketAddrs>(uuid: Uuid, remote_addr: T) -> Result<Self> {
        Ok(Self {
            uuid,
            udp_socket: establish_connection(remote_addr).await?,
        })
    }

    /// Returns the [`Uuid`] this [`Client`] instance was created with.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Returns the [`UdpSocket`] this client is bound to.
    pub fn udp_socket(&self) -> &UdpSocket {
        &self.udp_socket
    }

    /// Writes the message buffer to the [`Client`]'s underlying [`UdpSocket`].
    pub async fn send_message(&self, msg_buf: &[u8]) -> Result<usize> {
        Ok(self.udp_socket.send(msg_buf).await.map_err(ClientError::SendError)?)
    }
}

///
/// Establises a connection* with a remote address
///
/// # Behavior
/// Binds to local `[::]:0` address in order to be able to listen for incoming messages.
/// The function then automaticly connects* to the specified remote address.
///
/// # Error
/// Returns an error if it failed to bind to the local address, or failed to resolve remote address from the argument.
///
/// ***Udp is actually connectionless, please refer to [`UdpSocket::connect`] for its behavior.**
///
pub async fn establish_connection<T: ToSocketAddrs>(remote_addr: T) -> Result<UdpSocket> {
    let udp_socket = UdpSocket::bind("[::]:0").await.map_err(ClientError::BindError)?;

    udp_socket.connect(remote_addr).await.map_err(ClientError::ConnectionError)?;

    Ok(udp_socket)
}

/// Client error variant definition.
#[derive(thiserror::Error, Debug)]
pub enum ClientError {
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

type Result<T> = ::std::result::Result<T, ClientError>;