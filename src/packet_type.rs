use network_tcp_packet::impl_to_bytes;
use network_tcp_packet::traits::to_le_bytes::ToLeBytes;

#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u8)]
pub(crate) enum PacketType {
    NewFile,
    FileChunk,
    FileEnd,
}

impl_to_bytes!(PacketType, |this| {
   let byte = this as u8;
    [byte].to_vec()
});