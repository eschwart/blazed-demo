use crate::*;
use bincode::deserialize;
use packet_enum::*;
use std::net::{SocketAddr, ToSocketAddrs};

pub fn recv<'a, K: AsPacketKind, T: AsPacketRecv<'a, K>>(
    buf: &'a [u8],
    kind: K,
) -> BlazedResult<T> {
    let packet = deserialize::<T>(buf)?;

    if !kind.contains(packet.kind()) {
        return Err(BlazedError::Packet(PacketError::unexpected(
            packet.kind(),
            kind,
        )));
    }
    Ok(packet)
}

/// Retrieve a default socket with specified port number.
pub fn get_socket_addr(port: u16) -> SocketAddr {
    ("127.0.0.1", port)
        .to_socket_addrs()
        .expect("Failed to retrieve socket address(s)")
        .next()
        .expect("No available socket address(s)")
}
