use super::*;
use crate::*;

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream, UdpSocket},
};

use bincode::serialize;
use packet_enum::*;

pub trait UdpConn {
    fn socket(&self) -> &UdpSocket;

    fn send_to(&self, packet: &impl AsPacketSend, addr: &SocketAddr) -> BlazedResult<usize> {
        let bytes = serialize(packet)?;
        self.socket().send_to(&bytes, addr).map_err(Into::into)
    }

    fn recv_from<'a, K: AsPacketKind, T: AsPacketRecv<'a, K>>(
        &self,
        buf: &'a mut [u8],
        kind: K,
    ) -> BlazedResult<(T, SocketAddr)> {
        let (n, addr) = self.socket().recv_from(buf)?;
        let packet = recv(&buf[..n], kind)?;
        Ok((packet, addr))
    }
}

pub trait TcpConn {
    fn stream(&self) -> &TcpStream;

    fn send(&self, packet: &impl AsPacketSend) -> BlazedResult<()> {
        let bytes = serialize(packet)?;
        self.stream().write_all(&bytes).map_err(Into::into)
    }

    fn recv<'a, K: AsPacketKind, T: AsPacketRecv<'a, K>, const N: usize>(
        &self,
        buf: &'a mut [u8; N],
        kind: K,
    ) -> BlazedResult<T> {
        let bytes = self.stream().read(buf)?;
        recv(&buf[..bytes], kind)
    }
}
