//! Provides functions and helpers for the server side of the Voip service.
use super::{Result, UdpError};
use crate::{
    packet::{VoipHeader, VoipPacket},
    MTU_MAX_PACKET_SIZE,
};
use parking_lot::Mutex;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::UdpSocket,
    select,
    sync::mpsc::{channel, Receiver, Sender},
};
use tokio_util::sync::CancellationToken;
use tracing::{event, Level};

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
    inbound_message_receiver: Receiver<(VoipHeader, Vec<u8>, SocketAddr)>,

    /// This local channel receives messages which will be sent to listening clients at their remote addresses.
    outbound_message_sender: Sender<VoipPacket>,
}

#[derive(Debug, Default, Clone)]
/// Client list type definition.
pub struct ClientList(Arc<Mutex<Vec<SocketAddr>>>);

impl ClientList {
    /// **Will block if the underlying mutex is already locked by another thread.**
    ///
    /// Removes the specified [`SocketAddr`] from the client list.
    /// The removed item is returned as an [`Option<SocketAddr>`].
    ///
    /// If the item is not found [`None`] is returned.
    pub fn remove(&self, key: &SocketAddr) -> Option<SocketAddr> {
        let mut list = self.0.lock();

        list.iter()
            .position(|socket_addr| *socket_addr == *key)
            .map(|pos| list.swap_remove(pos))
    }
}

impl Server {
    /// Creates a new [`Server`] instance, and bind to the local IPV6 address with the given port.
    pub async fn new(port: u32) -> Result<Self> {
        let socket_handle = UdpSocket::bind(format!("[::]:{port}"))
            .await
            .map_err(UdpError::BindError)?;

        let (outbound_message_sender, mut outbound_message_receiver) = channel::<VoipPacket>(255);
        let (inbound_message_sender, inbound_message_receiver) =
            channel::<(VoipHeader, Vec<u8>, SocketAddr)>(255);
        let cancellation_token = CancellationToken::new();
        let client_list = ClientList::default();
        let client_list_clone = client_list.clone();
        let cancellation_token_clone = cancellation_token.clone();

        tokio::spawn(async move {
            loop {
                let client_list = client_list_clone.clone();

                //Create buffer for reading incoming messages
                let mut buf = Vec::with_capacity(8);

                select! {
                    //Await receving said amounts of bytes
                    incoming_bytes = socket_handle.recv_from(&mut buf) => {
                        match incoming_bytes {
                            Ok((_byte_count, socket_addr)) => {
                                let body_length = usize::from_be_bytes(buf.try_into().unwrap());

                                //Check for invalid messages
                                if body_length > MTU_MAX_PACKET_SIZE {
                                    //Log error
                                    event!(Level::ERROR, "Message header with too large length: {body_length}. Discarding message.");

                                    //If an inavlid message was provided discard it, to avoid overflowing buffer sizes
                                    continue;
                                }

                                //This cannot block as the header and the body is included in one message
                                let mut body_buf = Vec::with_capacity(body_length);

                                //Read from UdpSocket
                                socket_handle.recv(&mut body_buf).await.unwrap();

                                //Try serializing the bytes
                                match rmp_serde::from_slice::<VoipHeader>(&body_buf) {
                                    Ok(voip_header) => {
                                        let voip_body_length = match voip_header.voip_message_type() {
                                            crate::packet::VoipMessageType::VoiceMessage(length) => length,
                                            crate::packet::VoipMessageType::VideoMessage(length) => length,
                                        };

                                        //Create voip body buf allocate the length by fetching the header
                                        let mut voip_body_buf = Vec::with_capacity(*voip_body_length as usize);

                                        //Read the body into the buffer
                                        socket_handle.recv(&mut voip_body_buf).await.unwrap();

                                        //Send the serialized message through the channel
                                        inbound_message_sender.send((voip_header, voip_body_buf, socket_addr)).await.unwrap();
                                    },
                                    Err(err) => {
                                        event!(Level::ERROR, "Failed to deserialize a VoipPacket: {err}");
                                    },
                                }

                            },
                            Err(err) => {
                                event!(Level::ERROR, "Failed to receive message: {err}");
                            },
                        }
                    }

                    //Await outbound channel request
                    Some(outgoing_message) = outbound_message_receiver.recv() => {
                        //Clone the client list becasue it doesnt implement Send
                        let client_list_clone = client_list.0.lock().clone();

                        //Iter over all the remote_addresses and echo back the VoipPacket to everyone.
                        for remote_addr in client_list_clone.iter() {
                            //Send the VoipPacket to the remote address
                            socket_handle.send_to(outgoing_message.inner(), remote_addr).await.unwrap();
                        }
                    }

                    //Await thread cancellation
                    _ = cancellation_token_clone.cancelled() => break,
                }
            }
        });

        Ok(Self {
            connected_clients: client_list,
            inbound_message_receiver,
            cancellation_token,
            outbound_message_sender,
        })
    }

    /// Gets the incoming message receiver ([`Receiver<VoipPacket>`]) handle.
    /// This is created at the instance creation of [`Server`].
    /// The server listener threads has ownership of the sender, and sends every incoming message to the receiver.
    pub fn message_receiver(&mut self) -> &mut Receiver<(VoipHeader, Vec<u8>, SocketAddr)> {
        &mut self.inbound_message_receiver
    }

    /// Server thread cancellation token ([`CancellationToken`]) for shutting down the server.
    /// This can be cancelled in a sync environment.
    pub fn cancellation_token(&self) -> &CancellationToken {
        &self.cancellation_token
    }

    /// This gets the list of [`SocketAddr`]s which the UdpSocket should reply to.
    pub fn get_reply_to_list_mut(&self) -> Arc<Mutex<Vec<SocketAddr>>> {
        self.connected_clients.0.clone()
    }

    /// Replies to all of the [`SocketAddr`]-es specified in `self.connected_clients` through the [`UdpSocket`] the server is bound to.
    /// Sends the [`VoipPacket`] through a channel, which the server async thread is awaiting.
    pub async fn reply_to_clients(
        &self,
        voip_packet: VoipPacket,
    ) -> std::result::Result<(), tokio::sync::mpsc::error::SendError<VoipPacket>> {
        self.outbound_message_sender.send(voip_packet).await
    }
}
