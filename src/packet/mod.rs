//!
//!  A feature provides functions and abstractions for creating for sending packets.
//!

use uuid::Uuid;

/// Voip message variant type definition.
/// This enum contains the message variants the [`VoipPacket`] can contain.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum VoipMessageType {
    /// This message type contains the length of the data of an Audio recording.
    #[cfg(feature = "voice")]
    VoiceMessage(u64),

    /// This message type contains the length of the data of an Image.
    #[cfg(feature = "video")]
    VideoMessage(u64),
}

///
///  Struct definition for a Voip packet.
///
/// This Packet can contain a [`VoipMessageType::VoiceMessage`] or a [`VoipMessageType::VideoMessage`], with the author's [`Uuid`].
///
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct VoipHeader {
    /// The [`VoipMessageType`] of this packet.
    /// Can either be a Voice packet or a Video packet.
    voip_message_type: VoipMessageType,

    /// The author of this packet.
    /// This can be used to identify the sender of the [`VoipPacket`].
    author: Uuid,
}

/// Wrapper type for a buffer.
#[derive(Debug)]
pub struct VoipPacket(Vec<u8>);

impl VoipPacket {
    /// Returns the inner buffer of this packet.
    pub fn inner(&self) -> &[u8] {
        &self.0
    }
}

impl VoipHeader {
    /// Creates a new [`VoipPacket`] instance.
    pub fn new(voip_message_type: VoipMessageType, author: Uuid) -> Self {
        Self {
            voip_message_type,
            author,
        }
    }

    ///
    /// Creates a message buffer from a VoipPacket and the actual data.
    ///
    /// You must ensure that you are sending the correct set of bytes, matching the [VoipPacket::voip_message_type]'s variant.
    ///    
    pub fn create_message_buffer(
        &self,
        data: &[u8],
    ) -> Result<VoipPacket, rmp_serde::encode::Error> {
        //Create buffer
        let mut buffer: Vec<u8> = vec![];

        //Serialize header
        let serialized_packet = rmp_serde::to_vec(self)?;

        //Push length of the message
        buffer.extend((serialized_packet.len() + data.len()).to_be_bytes());

        //Push serialized VoipPacket
        buffer.extend(serialized_packet);

        //Push data
        buffer.extend(data);

        Ok(VoipPacket(buffer))
    }

    /// Fetches the [`VoipMessageType`] of the [`VoipHeader`].
    pub fn voip_message_type(&self) -> &VoipMessageType {
        &self.voip_message_type
    }
}
