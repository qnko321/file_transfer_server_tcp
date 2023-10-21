mod packet_type;
mod reader_controller;
mod logger;
mod sender;
mod receiver;
mod file_transfer;

use std::fs::File;
use std::os::windows::fs::FileExt;
use lazy_static::lazy_static;
use log::{error, info};
use network_tcp_packet::packet::packet::TcpPacket;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::spawn;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use uuid::{Uuid};
use std::collections::HashMap;
use std::path::Path;
use sha256::try_digest;
use crate::file_transfer::send_file;
use crate::logger::setup_logger;
use crate::packet_type::PacketType;
use crate::reader_controller::{ReaderController, ReaderState};
use crate::receiver::start_receiver;
use crate::sender::start_sender;

type Packet = TcpPacket<PacketType>;

const MAX_FILE_READ_LENGTH: usize = 50_000;

lazy_static! {
    static ref FILE_SENDERS: Mutex<HashMap<Uuid, mpsc::Sender<(u64, Vec<u8>)>>> = {
        Mutex::new(HashMap::new())
    };
}

#[tokio::main]
async fn main() {
    setup_logger().unwrap();
    let stream = TcpStream::connect("127.0.0.1:8082").await.unwrap();
    let (reader_half, writer_half) = tokio::io::split(stream);

    let (
        r_packet_sender,
        mut r_packet_receiver
    ) = mpsc::unbounded_channel::<Packet>();

    let (
        s_packet_sender,
        s_packet_receiver
    ) = mpsc::unbounded_channel::<Packet>();

    start_receiver(reader_half, r_packet_sender);
    start_sender(writer_half, s_packet_receiver);

    send_file("./test.txt", "test.txt", s_packet_sender);

    loop {
        let packet = match r_packet_receiver.recv().await {
            None => break,
            Some(packet) => packet
        };

        handle_packet(packet).await;
    }
}

async fn handle_packet(mut packet: Packet) {
    match packet.get_type() {
        PacketType::NewFile => {
            let _path = packet.read_string();
            let name = packet.read_string();
            let checksum = packet.read_string();
            let uuid = Uuid::from_slice(&packet.read_vector::<u8>()).unwrap();

            let local_path = format!(".//received\\{}", name);

            let (
                file_chunk_sender,
                mut file_chunk_receiver,
            ) = mpsc::channel::<(u64, Vec<u8>)>(10);

            FILE_SENDERS.lock().await.insert(uuid, file_chunk_sender);

            // write to file thread
            spawn(async move {
                let file = File::create(local_path.clone()).unwrap();

                loop {
                    let (offset, chunk) = match file_chunk_receiver.recv().await {
                        Some(data) => data,
                        None => {
                            let path2 = Path::new(&local_path);
                            let generated_checksum = try_digest(path2).unwrap();

                            if generated_checksum == checksum {
                                info!("Received a file successfully.");
                            } else {
                                error!("Received the full file, but its corrupted!");
                            }

                            break;
                        }
                    };

                    file.seek_write(&chunk, offset as u64).unwrap();
                }
            });
        }
        PacketType::FileChunk => {
            let uuid_bytes = packet.read_vector::<u8>();
            let uuid = Uuid::from_slice(&uuid_bytes).unwrap();

            let offset = packet.read::<u64>();
            let bytes = packet.read_vector::<u8>();

            FILE_SENDERS.lock().await.get(&uuid).unwrap().send((offset, bytes)).await.unwrap();
        }
        PacketType::FileEnd => {
            let uuid_bytes = packet.read_vector::<u8>();
            let uuid = Uuid::from_slice(&uuid_bytes).unwrap();

            FILE_SENDERS.lock().await.remove(&uuid).unwrap();
        }
    }
}