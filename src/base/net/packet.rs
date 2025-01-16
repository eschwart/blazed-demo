use crate::*;
use packet_enum::*;
use std::fmt::Debug;

#[derive(Clone, Copy, Debug)]
pub struct ClientHandshake;

#[derive(Clone, Copy, Debug)]
pub struct ServerHandshake(Id);

impl ServerHandshake {
    pub const fn id(&self) -> Id {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Handshake {
    secret: [u8; 3],
    is_client: Option<Id>,
}

impl Handshake {
    // TODO - improve this (arbitrary for now)
    const SECRET: [u8; 3] = [1, 0, 1];

    const fn new(is_client: Option<Id>) -> Handshake {
        Self {
            secret: Self::SECRET,
            is_client,
        }
    }

    pub const fn client() -> Handshake {
        Self::new(None)
    }

    pub const fn server(id: Id) -> Handshake {
        Self::new(Some(id))
    }

    pub const fn verify(&self) -> BlazedResult<()> {
        if let Self::SECRET = self.secret {
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
pub enum Mouse {
    Wheel { precise_y: f32 },
    Motion { xrel: i32, yrel: i32 },
}

pub type Keybaord = Flags;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Input {
    Mouse(Mouse),
    Keyboard(Keybaord),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PacketEnum)]
pub enum Packet {
    // initialization
    Handshake { handshake: Handshake },

    // client-related
    Input { input: Input },

    // object-related
    AddObj { data: ObjectData },
    RemObj { id: Id },
    UptObj { data: ObjectData },

    // misc functionality
    Flush,
    Ping,
}

impl Packet {
    fn into_handshake(self) -> BlazedResult<Handshake> {
        match self {
            Self::Handshake { handshake } => {
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
            Self::Input { input } => Ok(input),
            other => Err(BlazedError::Packet(PacketError::unexpected(
                PacketKind::Input,
                other.kind(),
            ))),
        }
    }
}
