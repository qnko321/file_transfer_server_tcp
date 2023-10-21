use tokio::io::{AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use crate::Packet;

pub(crate) fn start_sender(writer_half: WriteHalf<TcpStream>, mut s_packet_receiver: mpsc::UnboundedReceiver<Packet>) -> JoinHandle<()> {
    spawn(async move {
        let mut writer_half = writer_half;
        loop {
            let packet = match s_packet_receiver.recv().await {
                None => {
                    break;
                }
                Some(packet) => packet
            };

            let send_bytes = packet.to_bytes();
            writer_half.write_all(&send_bytes.len().to_le_bytes()).await.unwrap();
            writer_half.write_all(&send_bytes).await.unwrap();
        }
    })
}
