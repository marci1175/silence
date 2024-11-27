//! Provides functions and helpers for the server side of the Voip service.

use super::{Result, UdpError};
use crate::packet::VoipPacket;
use dashmap::DashMap;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::UdpSocket,
    select,
    sync::mpsc::{channel, Receiver},
};
use tokio_util::sync::CancellationToken;

///
/// Server instance type definition.
///
/// The [`Server`] has helper functions implemented inorder to make the usage of a server easier.
///  
#[derive(Debug)]
pub struct Server {
    /// The currently connected clients' list.
    connected_clients: ClientList,

    /// The locally bound server's [`CancellationToken`].
    /// This can be used to shut down the server.
    cancellation_token: CancellationToken,

    /// The incoming message's channel.
    message_receiver: Receiver<VoipPacket>,
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
        let socket_handle = UdpSocket::bind(format!("[::]:{port}"))
            .await
            .map_err(UdpError::BindError)?;

        let (sender, receiver) = channel(255);
        let cancellation_token = CancellationToken::new();

        let cancellation_token_clone = cancellation_token.clone();

        tokio::spawn(async move {
            loop {
                //Create buffer for reading incoming messages
                let mut buf = vec![0; 8];

                select! {
                    //Poll receving said amounts of bytes
                    byte_count = socket_handle.recv(&mut buf) => {
                        match byte_count {
                            Ok(_) => {
                                //This cannot block as the header and the body is included in one message
                                let mut body_buf = vec![0; usize::from_be_bytes(buf.try_into().unwrap())];

                                //Read from UdpSocket
                                socket_handle.recv(&mut body_buf).await.unwrap();

                                //Try serializing the bytes
                                match rmp_serde::from_slice::<VoipPacket>(&body_buf) {
                                    Ok(voip_packet) => {
                                        sender.send(voip_packet).await.unwrap();
                                    },
                                    Err(_err) => {

                                    },
                                }

                            },
                            Err(_err) => {

                            },
                        }
                    }

                    //Poll thread cancellation
                    _ = cancellation_token_clone.cancelled() => break,
                }
            }
        });

        Ok(Self {
            connected_clients: ClientList::default(),
            message_receiver: receiver,
            cancellation_token,
        })
    }
}
