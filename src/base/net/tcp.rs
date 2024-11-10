use crate::*;

use std::net::{SocketAddr, TcpListener, TcpStream};

#[derive(Debug)]
pub struct TcpClient {
    inner: TcpStream,
}

impl TcpClient {
    pub fn new(addr: SocketAddr) -> BlazedResult<Self> {
        let inner = TcpStream::connect(addr)?;
        Ok(Self { inner })
    }

    pub fn try_clone(&self) -> BlazedResult<Self> {
        let inner = self.inner.try_clone()?;
        Ok(Self { inner })
    }
}

impl TcpConn for TcpClient {
    fn stream(&self) -> &TcpStream {
        &self.inner
    }
}

#[derive(Debug)]
pub struct TcpServer {
    inner: TcpListener,
}

impl TcpServer {
    pub fn new(addr: SocketAddr) -> BlazedResult<Self> {
        let inner = TcpListener::bind(addr)?;
        Ok(Self { inner })
    }

    pub fn incoming(&self) -> impl Iterator<Item = TcpClient> + '_ {
        self.inner.incoming().filter_map(|s| {
            if let Ok(inner) = s {
                Some(TcpClient { inner })
            } else {
                None
            }
        })
    }
}
