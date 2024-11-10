use crate::*;

use std::fmt::Debug;

use base::{cam::*, err::*};
use packet_enum::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug)]
pub struct ClientHandshake;

#[derive(Clone, Copy, Debug)]
pub struct ServerHandshake(u8);

impl ServerHandshake {
    pub const fn id(&self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Handshake {
    secret: [u8; 3],
    is_client: Option<u8>,
}

impl Handshake {
    // maybe generate random hash at build-time
    const SECRET: [u8; 3] = [1, 0, 1];

    const fn new(is_client: Option<u8>) -> Handshake {
        Self {
            secret: Self::SECRET,
            is_client,
        }
    }

    pub const fn client() -> Handshake {
        Self::new(None)
    }

    pub const fn server(id: u8) -> Handshake {
        Self::new(Some(id))
    }

    pub fn verify(&self) -> BlazedResult<()> {
        if self.secret == Self::SECRET {
            Ok(())
        } else {
            Err(BlazedError::Packet(PacketError::Handshake(
                HandshakeError::InvalidContent,
            )))
        }
    }

    pub const fn into_client(self) -> Option<ClientHandshake> {
        if self.is_client.is_none() {
            Some(ClientHandshake)
        } else {
            None
        }
    }

    pub const fn into_server(self) -> Option<ServerHandshake> {
        if let Some(id) = self.is_client {
            Some(ServerHandshake(id))
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Player {
    id: u8,
    attr: CameraAttr,
}

impl Player {
    pub const fn new(id: u8) -> Self {
        Self {
            id,
            attr: CameraAttr::new(),
        }
    }

    pub const fn id(&self) -> u8 {
        self.id
    }

    pub const fn attr(&self) -> CameraAttr {
        self.attr
    }

    pub fn attr_mut(&mut self) -> &mut CameraAttr {
        &mut self.attr
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Input {
    Mouse { xrel: i32, yrel: i32 },
    Keyboard { keys: Flags },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PacketEnum)]
pub enum Packet {
    Handshake(Handshake),
    Input(Input),
    Player(Player),
    Remove(u8),
    Flush,
    Ping,
}

impl Packet {
    fn into_handshake(self) -> BlazedResult<Handshake> {
        match self {
            Self::Handshake(handshake) => {
                handshake.verify()?;
                Ok(handshake)
            }
            other => Err(BlazedError::Packet(PacketError::unexpected(
                PacketKind::Handshake,
                other.kind(),
            ))),
        }
    }

    pub fn into_client_handshake(self) -> BlazedResult<ClientHandshake> {
        self.into_handshake()?
            .into_client()
            .ok_or(BlazedError::Packet(PacketError::Handshake(
                HandshakeError::InvalidType,
            )))
    }

    pub fn into_server_handshake(self) -> BlazedResult<ServerHandshake> {
        self.into_handshake()?
            .into_server()
            .ok_or(BlazedError::Packet(PacketError::Handshake(
                HandshakeError::InvalidType,
            )))
    }

    pub fn into_input(self) -> BlazedResult<Input> {
        match self {
            Self::Input(input) => Ok(input),
            other => Err(BlazedError::Packet(PacketError::unexpected(
                PacketKind::Input,
                other.kind(),
            ))),
        }
    }

    pub fn into_player(self) -> BlazedResult<Player> {
        match self {
            Self::Player(update) => Ok(update),
            other => Err(BlazedError::Packet(PacketError::unexpected(
                PacketKind::Player,
                other.kind(),
            ))),
        }
    }
}
