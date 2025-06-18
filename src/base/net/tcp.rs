use crate::*;
use std::{
    net::{SocketAddr, TcpListener, TcpStream},
    ops::Deref,
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct TcpClient {
    inner: Arc<TcpStream>,
}

impl TcpClient {
    pub fn new(addr: SocketAddr) -> BlazedResult<Self> {
        let inner = Arc::new(TcpStream::connect(addr)?);
        Ok(Self { inner })
    }
}

impl Deref for TcpClient {
    type Target = TcpStream;

    fn deref(&self) -> &Self::Target {
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
            if let Ok(stream) = s {
                let inner = Arc::new(stream);
                Some(TcpClient { inner })
            } else {
                None
            }
        })
    }
}
