use crate::*;
use std::{
    net::{SocketAddr, UdpSocket},
    ops::Deref,
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct UdpClient {
    inner: Arc<UdpSocket>,
}

impl UdpClient {
    pub fn new(local_addr: SocketAddr, remote_addr: SocketAddr) -> BlazedResult<Self> {
        let inner = UdpSocket::bind(local_addr)?;
        inner.connect(remote_addr)?;
        let inner = Arc::new(inner);
        Ok(Self { inner })
    }

    pub fn send(&mut self, buf: &[u8]) -> BlazedResult {
        self.inner.send(buf).map(|_| ()).map_err(Into::into)
    }

    pub fn recv(&self, buf: &mut [u8]) -> BlazedResult<usize> {
        self.inner.recv(buf).map_err(Into::into)
    }
}

impl Deref for UdpClient {
    type Target = UdpSocket;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Clone, Debug)]
pub struct UdpServer {
    inner: Arc<UdpSocket>,
}

impl UdpServer {
    pub fn new(addr: SocketAddr) -> BlazedResult<Self> {
        let inner = Arc::new(UdpSocket::bind(addr)?);
        Ok(Self { inner })
    }
}

impl Deref for UdpServer {
    type Target = UdpSocket;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
