use crate::*;

use crossbeam_channel::{RecvError, SendError, TryRecvError, TrySendError};
use packet_enum::AsPacketKind;
use strum::Display;

pub type BlazedResult<T = (), E = BlazedError> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug, Display)]
pub enum HandshakeError {
    InvalidContent,
    InvalidType,
    Unknown,
}

#[derive(thiserror::Error, Debug)]
pub enum PacketError {
    #[error("Handshake({0})")]
    Handshake(#[from] HandshakeError),

    #[error("Expected {lhs:?}, found {rhs:?}")]
    Unexpected { lhs: String, rhs: String },
}

impl PacketError {
    pub fn unexpected<K: AsPacketKind>(lhs: K, rhs: K) -> Self {
        Self::Unexpected {
            lhs: format!("{:?}", lhs),
            rhs: format!("{:?}", rhs),
        }
    }
}

#[derive(thiserror::Error, Debug, Display)]
pub enum SyncError {
    Send,
    TrySend,
    Recv,
    TryRecv,
    Disconnected,
}

impl<T> From<SendError<T>> for SyncError {
    fn from(_: SendError<T>) -> Self {
        Self::Send
    }
}

impl<T> From<TrySendError<T>> for SyncError {
    fn from(value: TrySendError<T>) -> Self {
        if let TrySendError::Disconnected(..) = value {
            Self::Disconnected
        } else {
            Self::TrySend
        }
    }
}

impl From<RecvError> for SyncError {
    fn from(_: RecvError) -> Self {
        Self::Recv
    }
}

impl From<TryRecvError> for SyncError {
    fn from(value: TryRecvError) -> Self {
        if let TryRecvError::Disconnected = value {
            Self::Disconnected
        } else {
            Self::TryRecv
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum BlazedError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Signal(#[from] ctrlc::Error),

    #[error(transparent)]
    Json(#[from] Box<bincode::ErrorKind>),

    #[error(transparent)]
    Packet(#[from] PacketError),

    #[error(transparent)]
    Sync(SyncError),

    #[error("{0}")]
    Misc(String),

    #[error("An unexpected error has ocurred")]
    Unknown,

    #[error("Infallible")]
    Infallible,
}

impl<T: Into<SyncError>> From<T> for BlazedError {
    fn from(value: T) -> Self {
        Self::Sync(value.into())
    }
}

impl From<&str> for BlazedError {
    fn from(value: &str) -> Self {
        Self::Misc(value.to_string())
    }
}

impl From<String> for BlazedError {
    fn from(value: String) -> Self {
        Self::Misc(value)
    }
}

impl From<Box<dyn std::any::Any + Send>> for BlazedError {
    fn from(value: Box<dyn std::any::Any + Send>) -> Self {
        match value.downcast::<&str>() {
            Ok(s) => Self::Misc(s.to_string()),
            Err(_) => Self::Unknown,
        }
    }
}
