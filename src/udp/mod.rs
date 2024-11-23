//!
//!  This feature provides functions and abstractions for sending both Voice and Video packets.
//! 

pub mod client;
pub mod server;

pub enum VoipMessage {
    VoiceMessage(VoicePacket),
    VideoMessage(VideoPacket),
}

pub struct VoicePacket {
    bytes: Vec<u8>,
}

pub struct VideoPacket {
    bytes: Vec<u8>,
}