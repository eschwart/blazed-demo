use crate::*;
use enum_unit::*;
use std::ops::{Deref, DerefMut};

#[derive(Clone, Copy, Debug)]
pub struct Player {
    id: Id,
    data: PlayerData,
}

impl Player {
    pub fn new(id: Id, pos: Vector) -> Self {
        let data = PlayerData::new(pos);
        Self { id, data }
    }

    pub const fn id(&self) -> Id {
        self.id
    }

    pub const fn data(&self) -> PlayerData {
        self.data
    }
}

impl Deref for Player {
    type Target = PlayerData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for Player {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[derive(Debug)]
pub struct PlayerRef<'a> {
    id: Id,
    data: &'a PlayerData,
}

impl PlayerRef<'_> {
    pub const fn id(&self) -> Id {
        self.id
    }
}

impl Deref for PlayerRef<'_> {
    type Target = PlayerData;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

#[derive(Debug)]
pub struct PlayerMut<'a> {
    id: Id,
    data: &'a mut PlayerData,
}

impl PlayerMut<'_> {
    pub const fn id(&self) -> Id {
        self.id
    }
}

impl Deref for PlayerMut<'_> {
    type Target = PlayerData;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl DerefMut for PlayerMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct PlayerData {
    attr: CameraAttr,
}

impl PlayerData {
    pub fn new(pos: Vector) -> Self {
        Self {
            attr: CameraAttr::new(pos),
        }
    }

    pub const fn pos(&self) -> &Vector {
        &self.attr.eye
    }

    pub fn pos_mut(&mut self) -> &mut Vector {
        &mut self.attr.eye
    }

    pub const fn dim(&self) -> &Vector {
        &DIAGONAL
    }

    pub const fn attr(&self) -> CameraAttr {
        self.attr
    }

    pub fn attr_mut(&mut self) -> &mut CameraAttr {
        &mut self.attr
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Basic {
    id: Id,
    data: BasicData,
}

impl Basic {
    pub const fn id(&self) -> Id {
        self.id
    }

    pub const fn data(&self) -> BasicData {
        self.data
    }
}

impl Deref for Basic {
    type Target = BasicData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[derive(Debug)]
pub struct BasicMut<'a> {
    id: Id,
    data: &'a mut BasicData,
}

#[derive(Debug)]
pub struct BasicRef<'a> {
    id: Id,
    data: &'a BasicData,
}

impl Deref for BasicRef<'_> {
    type Target = BasicData;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl Deref for BasicMut<'_> {
    type Target = BasicData;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl DerefMut for BasicMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct BasicData {
    pos: Vector,
    dim: Vector,
}

impl BasicData {
    pub const fn new(pos: Vector, dim: Vector) -> Self {
        Self { pos, dim }
    }

    pub const fn pos(&self) -> &Vector {
        &self.pos
    }

    pub const fn dim(&self) -> &Vector {
        &self.dim
    }

    pub fn pos_mut(&mut self) -> &mut Vector {
        &mut self.pos
    }

    pub fn dim_mut(&mut self) -> &mut Vector {
        &mut self.dim
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, EnumUnit)]
pub enum RawObjectData {
    Player(PlayerData),
    Basic(BasicData),
}

impl RawObjectData {
    pub const fn pos(&self) -> &Vector {
        match self {
            RawObjectData::Player(ref data) => data.pos(),
            RawObjectData::Basic(ref data) => data.pos(),
        }
    }

    pub const fn dim(&self) -> &Vector {
        match self {
            RawObjectData::Player(data) => data.dim(),
            RawObjectData::Basic(data) => data.dim(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Color {
    inner: [f32; 4],
    emits: bool,
}

impl Color {
    pub const fn new(inner: [f32; 4], emits: bool) -> Self {
        Self { inner, emits }
    }

    pub const fn emits(&self) -> bool {
        self.emits
    }

    pub const fn alpha(&self) -> f32 {
        self.inner[3]
    }

    pub const fn is_opaque(alpha: f32) -> bool {
        alpha as i32 == 1
    }
}

impl Deref for Color {
    type Target = [f32];

    fn deref(&self) -> &Self::Target {
        self.inner.as_slice()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Transformations {
    translation: Translation,
    rotation: UnitQuaternion,
    scaling: Scale,
    model: Matrix,
}

impl Transformations {
    pub fn new(pos: Vector, dim: Vector) -> Self {
        let translation = Translation::from(pos);
        let scaling = Scale::from(dim);

        Self {
            translation,
            scaling,
            ..Default::default()
        }
    }
}

impl Default for Transformations {
    fn default() -> Self {
        let translation = Translation::identity();
        let rotation = UnitQuaternion::identity();
        let scaling = Scale::identity();
        let model = Matrix::identity();

        Self {
            translation,
            rotation,
            scaling,
            model,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ObjectData {
    id: Id,
    color: Color,
    data: RawObjectData,

    #[serde(skip)]
    transform: Transformations,
}

impl Deref for ObjectData {
    type Target = RawObjectData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl ObjectData {
    pub fn new(id: Id, color: Color, data: RawObjectData) -> Self {
        let transform = Transformations::new(*data.pos(), *data.dim());
        Self {
            id,
            color,
            data,
            transform,
        }
    }

    pub const fn id(&self) -> Id {
        self.id
    }

    pub const fn color(&self) -> &[f32] {
        self.color.inner.as_slice()
    }

    pub const fn alpha(&self) -> f32 {
        self.color.alpha()
    }

    pub const fn is_light(&self) -> bool {
        self.color.emits
    }

    pub const fn player(&self) -> Option<Player> {
        let id = self.id();

        if let RawObjectData::Player(data) = self.data {
            Some(Player { id, data })
        } else {
            None
        }
    }

    pub fn player_ref(&self) -> Option<PlayerRef> {
        let id = self.id();

        if let RawObjectData::Player(data) = &self.data {
            Some(PlayerRef { id, data })
        } else {
            None
        }
    }

    pub fn player_mut(&mut self) -> Option<PlayerMut> {
        let id = self.id();

        if let RawObjectData::Player(data) = &mut self.data {
            Some(PlayerMut { id, data })
        } else {
            None
        }
    }

    pub const fn basic(&self) -> Option<Basic> {
        let id = self.id();

        if let RawObjectData::Basic(data) = self.data {
            Some(Basic { id, data })
        } else {
            None
        }
    }

    pub fn basic_ref(&self) -> Option<BasicRef> {
        let id = self.id();

        if let RawObjectData::Basic(data) = &self.data {
            Some(BasicRef { id, data })
        } else {
            None
        }
    }

    pub fn basic_mut(&mut self) -> Option<BasicMut> {
        let id = self.id();

        if let RawObjectData::Basic(data) = &mut self.data {
            Some(BasicMut { id, data })
        } else {
            None
        }
    }

    pub fn translation(&self) -> &Translation {
        &self.transform.translation
    }

    pub fn translation_upt(&mut self) {
        self.transform.translation = Translation::from(*self.pos())
    }

    pub fn rotation(&self) -> &UnitQuaternion {
        &self.transform.rotation
    }

    pub fn rotation_upt(&mut self) {
        if let Some(p) = self.player() {
            let attr = p.attr();

            let yaw = attr.yaw.radians();
            let pitch = attr.pitch.radians();

            // rotation matrices for yaw and pitch
            let yaw_rotation = UnitQuaternion::from_axis_angle(&Y_AXIS_UNIT, -yaw);
            let pitch_rotation = UnitQuaternion::from_axis_angle(&X_AXIS_UNIT, pitch);

            // yaw and pitch into a single rotation
            self.transform.rotation = yaw_rotation * pitch_rotation
        }
    }

    pub fn scaling(&self) -> &Scale {
        &self.transform.scaling
    }

    pub fn scaling_upt(&mut self) {
        self.transform.scaling = Scale::from(*self.dim());
    }

    pub fn model(&self) -> &Matrix {
        &self.transform.model
    }

    pub fn model_upt(&mut self) {
        let t = self.translation();
        let r = self.rotation();
        let s = self.scaling();

        let iso = Isometry::from_parts(*t, *r).to_homogeneous();
        let model = iso * s.to_homogeneous();

        self.transform.model = model
    }

    pub fn transform_upt(&mut self) {
        self.translation_upt();
        self.rotation_upt();
        self.scaling_upt();
        self.model_upt();
    }
}
