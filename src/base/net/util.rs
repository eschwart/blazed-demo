use crate::*;

use bincode::deserialize;
use packet_enum::*;

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
