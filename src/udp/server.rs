//! Provides functions and helpers for the server side of the Voip service.

use std::{net::SocketAddr, sync::Arc};

use dashmap::DashMap;
use tokio::net::UdpSocket;

use super::{Result, UdpError};

///
/// Server instance type definition.
///
/// The [`Server`] has helper functions implemented inorder to make the usage of a server easier.
///  
#[derive(Debug)]
pub struct Server {
    /// The currently connected clients' list.
    connected_clients: ClientList,
    /// The locally bound server's handle.
    socket_handle: UdpSocket,
}

#[derive(Debug, Default)]
/// Client list type definition
pub struct ClientList(Arc<DashMap<SocketAddr, ()>>);

impl ClientList {
    /// Removes the client from the server via the socket address.
    /// This function also disconnects the user to prevent more data from being sent.
    pub fn remove(&self, key: &SocketAddr) -> Option<(SocketAddr, ())> {
        self.0.remove(key)
    }
}

impl Server {
    /// Creates a new [`Server`] instance, and bind to the local IPV6 address with the given port.
    pub async fn new(port: u32) -> Result<Self> {
        Ok(
            Self {
                connected_clients: ClientList::default(),
                socket_handle: UdpSocket::bind(format!("[::]:{port}")).await.map_err(UdpError::BindError)?,
            }
        )
    }
}