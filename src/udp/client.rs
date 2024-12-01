//! Provides functions and helpers for the client side of the Voip service.
use std::collections::VecDeque;
use std::sync::Arc;

use super::Result;
use super::UdpError;
use crate::packet::VoipHeader;
use crate::packet::VoipMessageType;
use crate::packet::VoipPacket;
use crate::MTU_MAX_PACKET_SIZE;
use parking_lot::Mutex;
use silence_core::opus::encode::encode_samples_opus;
use silence_core::opus::opus::Encoder;
use tokio::net::{ToSocketAddrs, UdpSocket};
use tokio::select;
use tokio::sync::mpsc::channel;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tracing::event;
use tracing::Level;
use uuid::Uuid;

/// Client struct definition, mnade to simplify the usage of a client.
#[derive(Debug)]
pub struct Client {
    /// The unique identificator for this [`Client`] instance.
    uuid: Uuid,

    /// The receiver used to receive messages from the server.
    inbound_message_receiver: Receiver<(VoipHeader, Vec<u8>)>,

    /// This local channel sends messages which will be sent to the server.
    outbound_message_sender: Sender<VoipPacket>,
}

impl Client {
    /// Creates a new [`Client`] instance, automaticly sets up the [`UdpSocket`].
    pub async fn new<T: ToSocketAddrs>(uuid: Uuid, remote_addr: T) -> Result<Self> {
        //Create I/O channels
        let (outbound_message_sender, outbound_message_receiver) = channel::<VoipPacket>(255);
        let (inbound_message_sender, inbound_message_receiver) =
            channel::<(VoipHeader, Vec<u8>)>(255);

        //Bind UdpSocket to local address
        let socket_handle = establish_connection(remote_addr).await?;

        //Establish client service
        Self::create_client_service(
            socket_handle,
            inbound_message_sender,
            outbound_message_receiver,
        );

        Ok(Self {
            uuid,
            inbound_message_receiver,
            outbound_message_sender,
        })
    }

    /// Creates a new [`Client`] instance from an already existing [`UdpSocket`].
    pub async fn new_from_udp_socket(uuid: Uuid, socket_handle: UdpSocket) -> Result<Self> {
        //Create I/O channels
        let (outbound_message_sender, outbound_message_receiver) = channel::<VoipPacket>(255);
        let (inbound_message_sender, inbound_message_receiver) =
            channel::<(VoipHeader, Vec<u8>)>(255);

        //Establish client service
        Self::create_client_service(
            socket_handle,
            inbound_message_sender,
            outbound_message_receiver,
        );

        Ok(Self {
            uuid,
            inbound_message_receiver,
            outbound_message_sender,
        })
    }

    /// Returns the [`Uuid`] this [`Client`] instance was created with.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Writes the message buffer to the [`Client`]'s underlying [`UdpSocket`].
    pub fn message_sender(&mut self) -> &mut Sender<VoipPacket> {
        &mut self.outbound_message_sender
    }

    /// Gets the incoming message receiver ([`Receiver<VoipPacket>`]) handle.
    /// This is created at the instance creation of [`Server`].
    /// The server listener threads has ownership of the sender, and sends every incoming message to the receiver.
    pub fn message_receiver(&mut self) -> &mut Receiver<(VoipHeader, Vec<u8>)> {
        &mut self.inbound_message_receiver
    }

    fn create_client_service(
        socket_handle: UdpSocket,
        inbound_message_sender: Sender<(VoipHeader, Vec<u8>)>,
        mut outbound_message_receiver: Receiver<VoipPacket>,
    ) {
        tokio::spawn(async move {
            loop {
                let mut buf = Vec::with_capacity(8);

                select! {
                    //Await incoming messages from the server.
                    //If received send it through the `inbound_message_receiver`.
                    incoming_bytes = socket_handle.recv_from(&mut buf) => {
                        match incoming_bytes {
                            Ok((_byte_count, _socket_addr)) => {
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
                                        inbound_message_sender.send((voip_header, voip_body_buf)).await.unwrap();
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

                    //Await outgoing message requests from the user.
                    //If the channel receives a [`VoipPacket`] this function will send it to the connected [`SocketAddr`].
                    Some(outgoing_message) = outbound_message_receiver.recv() => {
                        //Send the VoipPacket to the remote address
                        socket_handle.send(outgoing_message.inner()).await.unwrap();
                    }
                }
            }
        });
    }

    /// Automaticly fetches the samples from the buffer, and sends them to the remote address
    pub async fn send_voice_packet(&self, encoder: Encoder, channels: silence_core::opus::opus::Channels, buffer: Arc<Mutex<VecDeque<f32>>>) -> anyhow::Result<()> {
        let mut sample_buf = vec![];
        while let Some(sample) = buffer.lock().pop_front() {
            sample_buf.push(sample);
        }

        let sound_packets = encode_samples_opus(encoder, &sample_buf, 20, channels)?;

        for sound_packet in sound_packets {
            self.outbound_message_sender.send(VoipHeader::new(VoipMessageType::VoiceMessage(1), self.uuid).create_message_buffer(&sound_packet.bytes)?).await?;
        }

        Ok(())
    }

    /// Creates a message manually, you can set the message_type and the bytes manually.
    /// Writes a [`VoipPacket`] to the client's underlying [`UdpSocket`].
    /// Creates a [`VoipPacket`] from the arguments passed in.
    pub async fn send_bytes(
        &self,
        voip_message_type: VoipMessageType,
        bytes: &mut dyn Iterator<Item = u8>,
    ) -> anyhow::Result<()> {
        // Collect the data
        let data: Vec<u8> = bytes.collect();

        // Create the VoipPacket
        let voip_packet =
            VoipHeader::new(voip_message_type, self.uuid).create_message_buffer(&data)?;

        if voip_packet.inner().len() > MTU_MAX_PACKET_SIZE {
            panic!("The manually constructed packet is too large.")
        }

        // Send it to the receving part
        self.outbound_message_sender.send(voip_packet).await?;

        Ok(())
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
async fn establish_connection<T: ToSocketAddrs>(remote_addr: T) -> Result<UdpSocket> {
    let udp_socket = UdpSocket::bind("[::]:0")
        .await
        .map_err(UdpError::BindError)?;

    udp_socket
        .connect(remote_addr)
        .await
        .map_err(UdpError::ConnectionError)?;

    Ok(udp_socket)
}
