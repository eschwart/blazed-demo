use super::*;
use crate::*;

use std::net::{SocketAddr, UdpSocket};

use bincode::serialize;
use packet_enum::{AsPacketKind, AsPacketRecv, AsPacketSend};

#[derive(Debug)]
pub struct UdpClient {
    inner: UdpSocket,
}

impl UdpClient {
    pub fn new(local_addr: SocketAddr, remote_addr: SocketAddr) -> BlazedResult<Self> {
        let inner = UdpSocket::bind(local_addr)?;
        inner.connect(remote_addr)?;
        Ok(Self { inner })
    }

    pub fn send(&mut self, packet: &impl AsPacketSend) -> BlazedResult<usize> {
        let bytes = serialize(packet)?;
        let n = self.inner.send(&bytes)?;
        Ok(n)
    }

    pub fn recv<'a, K: AsPacketKind, T: AsPacketRecv<'a, K>>(
        &self,
        buf: &'a mut [u8],
        kind: K,
    ) -> BlazedResult<T> {
        let n = self.inner.recv(buf)?;
        recv(&buf[..n], kind)
    }

    pub fn try_clone(&self) -> BlazedResult<Self> {
        let inner = self.inner.try_clone()?;
        Ok(Self { inner })
    }
}

impl UdpConn for UdpClient {
    fn socket(&self) -> &UdpSocket {
        &self.inner
    }
}

#[derive(Debug)]
pub struct UdpServer {
    inner: UdpSocket,
}

impl UdpServer {
    pub fn new(addr: SocketAddr) -> BlazedResult<Self> {
        let inner = UdpSocket::bind(addr)?;
        Ok(Self { inner })
    }

    pub fn try_clone(&self) -> BlazedResult<Self> {
        let inner = self.inner.try_clone()?;
        Ok(Self { inner })
    }
}

impl UdpConn for UdpServer {
    fn socket(&self) -> &UdpSocket {
        &self.inner
    }
}
