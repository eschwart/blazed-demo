use crate::*;

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Window: {0}")]
    Window(sdl2::video::WindowBuildError),

    #[error("Obj: {0}")]
    Obj(::obj::ObjError),

    #[error(transparent)]
    Blazed(BlazedError),
}

impl<T: Into<BlazedError>> From<T> for Error {
    fn from(value: T) -> Self {
        Self::Blazed(value.into())
    }
}
