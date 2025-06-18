use crate::*;
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream, UdpSocket},
    ops::Deref,
};

pub trait UdpConn {
    fn socket(&self) -> &UdpSocket;

    fn send_to(&self, buf: &[u8], addr: &SocketAddr) -> BlazedResult<usize> {
        self.socket().send_to(buf, addr).map_err(Into::into)
    }

    fn recv_from(&self, buf: &mut [u8]) -> BlazedResult<(usize, SocketAddr)> {
        self.socket().recv_from(buf).map_err(Into::into)
    }
}

impl<T: Deref<Target = UdpSocket>> UdpConn for T {
    fn socket(&self) -> &UdpSocket {
        self
    }
}

pub trait TcpConn {
    fn stream(&self) -> &TcpStream;

    fn send(&self, buf: &[u8]) -> BlazedResult {
        self.stream().write_all(buf).map_err(Into::into)
    }

    fn recv(&self, buf: &mut [u8]) -> BlazedResult<usize> {
        self.stream().read(buf).map_err(Into::into)
    }
}

impl<T: Deref<Target = TcpStream>> TcpConn for T {
    fn stream(&self) -> &TcpStream {
        self
    }
}
