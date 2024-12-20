#[cfg(test)]
mod test_functions {
    use uuid::Uuid;

    use crate::{
        packet::VoipHeader,
        udp::{client::Client, server::Server},
    };

    #[cfg(feature = "all")]
    #[tokio::test]
    async fn exchange_data() {
        tokio::spawn(async move {
            let mut server = Server::new(3004).await.unwrap();
            let msg_recv = server.message_receiver();

            //Wait for incoming message
            let (_packet, voip_body, _addr) = msg_recv.recv().await.unwrap();

            assert_eq!(voip_body, vec![1; 1]);
            server.get_reply_to_list_mut().insert(_addr);

            server
                .reply_to_clients(_packet.create_message_buffer(&voip_body).unwrap())
                .await
                .unwrap();
        });

        tokio::spawn(async move {
            let mut client = Client::new(Uuid::new_v4(), "[::1]:3004").await.unwrap();

            let packet = VoipHeader::new(
                crate::packet::VoipMessageType::VoiceMessage(1),
                client.uuid(),
            );

            let message_sender = client.message_sender();

            message_sender
                .send(packet.create_message_buffer(&[1; 1]).unwrap())
                .await
                .unwrap();
        });
    }
}
