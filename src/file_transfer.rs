#![allow(dead_code)]

use std::fs::File;
use std::os::windows::fs::FileExt;
use std::path::Path;
use log::error;
use sha256::try_digest;
use tokio::sync::mpsc;
use uuid::Uuid;
use walkdir::WalkDir;
use crate::{MAX_FILE_READ_LENGTH, Packet};
use crate::packet_type::PacketType;

pub(crate) fn send_folder(path: &str, s_packet_sender: mpsc::UnboundedSender<Packet>) {
    let walk_dir = WalkDir::new(path).max_depth(1);

    for entry in walk_dir {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            send_file(entry.path().to_str().unwrap(), entry.file_name().to_str().unwrap(), s_packet_sender.clone());
        }
    }
}

pub(crate) fn send_folder_all(path: &str, s_packet_sender: mpsc::UnboundedSender<Packet>) {
    for entry in WalkDir::new(path) {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            send_file(entry.path().to_str().unwrap(), entry.file_name().to_str().unwrap(), s_packet_sender.clone());
        }
    }
}

pub(crate) fn send_file(path: &str, name: &str, s_packet_sender: mpsc::UnboundedSender<Packet>) {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) => {
            error!("Failed to open file: {}", error);
            return;
        }
    };

    let file_path = Path::new(path);
    let checksum = match try_digest(file_path) {
        Ok(checksum) => checksum,
        Err(error) => {
            error!("Failed to generate checksum for file: {}", error);
            return;
        }
    };
    let uuid = Uuid::new_v4();
    let uuid_bytes = uuid.as_bytes();

    let mut new_file_packet = Packet::new(PacketType::NewFile);
    new_file_packet.write(path.to_string());
    new_file_packet.write(name.to_string());
    new_file_packet.write(checksum);
    new_file_packet.write_vector(uuid_bytes.to_vec());

    s_packet_sender.send(new_file_packet).unwrap();

    let mut file_buffer = vec![0u8; MAX_FILE_READ_LENGTH];
    let mut offset = 0;
    loop {
        let read_len = match file.seek_read(&mut file_buffer, offset) {
            Ok(len) => len,
            Err(error) => {
                error!("Failed to read from file: {}", error);
                return;
            }
        };

        if read_len == 0 {
            break;
        }

        let mut file_chunk_packet = Packet::new(PacketType::FileChunk);
        file_chunk_packet.write_vector(uuid_bytes.to_vec());
        file_chunk_packet.write(offset);
        file_chunk_packet.write_vector(file_buffer[0..read_len].to_vec());

        s_packet_sender.send(file_chunk_packet).unwrap();

        if read_len < MAX_FILE_READ_LENGTH {
            break;
        }

        offset += MAX_FILE_READ_LENGTH as u64;
    }

    let mut file_end_packet = Packet::new(PacketType::FileEnd);
    file_end_packet.write_vector::<u8>(uuid_bytes.to_vec());
    s_packet_sender.send(file_end_packet).unwrap();
}