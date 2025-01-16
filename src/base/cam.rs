use crate::*;
use serde::{Deserialize, Serialize};
use std::ops::{AddAssign, SubAssign};

pub type Point = nalgebra::Point3<f32>;
pub type Vector = nalgebra::Vector3<f32>;
pub type Matrix = nalgebra::Matrix4<f32>;
pub type Translation = nalgebra::Translation3<f32>;
pub type Rotation = nalgebra::Rotation3<f32>;
pub type Scale = nalgebra::Scale3<f32>;
pub type Perspective = nalgebra::Perspective3<f32>;
pub type UnitQuaternion = nalgebra::UnitQuaternion<f32>;
pub type Isometry = nalgebra::Isometry3<f32>;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Axis {
    inner: f32,
    radians: f32,
    sensitivity: f32,
}

impl Axis {
    const X_BOUND: f32 = 360.0;
    const Y_LOWER_BOUND: f32 = -89.0;
    const Y_UPPER_BOUND: f32 = 89.0;

    pub const fn new(sensitivity: f32) -> Self {
        Self {
            inner: 0.0,
            radians: 0.0,
            sensitivity,
        }
    }

    pub const fn radians(&self) -> f32 {
        self.radians
    }

    pub const fn sensitivity(&self) -> f32 {
        self.sensitivity
    }

    pub fn sensitivity_mut(&mut self) -> &mut f32 {
        &mut self.sensitivity
    }

    fn x_bound(&mut self) {
        self.inner = (self.inner + Self::X_BOUND) % Self::X_BOUND;
    }

    fn y_bound(&mut self) {
        self.inner = clamp_unchecked(self.inner, Self::Y_LOWER_BOUND, Self::Y_UPPER_BOUND);
    }

    fn update(&mut self) {
        self.radians = self.inner * RADIAN
    }
}

impl AddAssign<i32> for Axis {
    fn add_assign(&mut self, rhs: i32) {
        self.inner += rhs as f32 * self.sensitivity;
    }
}

impl SubAssign<i32> for Axis {
    fn sub_assign(&mut self, rhs: i32) {
        self.inner -= rhs as f32 * self.sensitivity;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CameraAttr {
    pub fov: f32,
    pub speed: f32,
    pub yaw: Axis,
    pub pitch: Axis,
    pub eye: Vector,
    pub target: Vector,
    pub up: Vector,
}

impl CameraAttr {
    pub fn new(pos: Vector) -> Self {
        let mut attr = Self {
            fov: 80.0,
            speed: 0.05,
            yaw: Axis::new(0.1),
            pitch: Axis::new(0.1),
            eye: pos,
            target: Vector::new(0.0, 0.0, -1.0),
            up: Vector::new(0.0, 1.0, 0.0),
        };

        // initial setup
        attr.look_at(0, 0);

        attr
    }

    pub fn upt_fov(&mut self, precise_y: f32) {
        self.fov = clamp_unchecked(self.fov - precise_y, 30.0, 110.0);
    }

    pub fn look_at(&mut self, xrel: i32, yrel: i32) {
        // incr/decr pitch/yaw values
        self.yaw += xrel;
        self.pitch -= yrel;

        // prevent overflow
        self.yaw.x_bound();

        // prevent unecessary vertical freedom
        self.pitch.y_bound();

        // update radian values
        self.yaw.update();
        self.pitch.update();

        // radian values of each axis
        let yaw_radians = self.yaw.radians;
        let pitch_radians = self.pitch.radians;

        let pitch_cos = pitch_radians.cos();

        // calculate new target values
        self.target.x = yaw_radians.sin() * pitch_cos;
        self.target.y = pitch_radians.sin();
        self.target.z = -yaw_radians.cos() * pitch_cos;

        // normalize
        self.target.normalize_mut();
    }

    pub fn input(&mut self, flags: Flags) {
        let mut target = self.target;
        target.y = 0.0;

        for key in flags.iter() {
            match key {
                // new
                Flags::W => self.eye += target.normalize() * self.speed,
                Flags::A => self.eye -= target.cross(&self.up).normalize() * self.speed,
                Flags::S => self.eye -= target.normalize() * self.speed,
                Flags::D => self.eye += target.cross(&self.up).normalize() * self.speed,

                Flags::SPACE => self.eye += self.up * self.speed,
                Flags::SHIFT => self.eye -= self.up * self.speed,

                _ => (),
            }
        }
    }
}

impl Default for CameraAttr {
    fn default() -> Self {
        Self::new(Vector::zeros())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawCamera {
    attr: CameraAttr,
    view: Matrix,
    projection: Perspective,
}

impl RawCamera {
    pub fn new((w, h): (u32, u32)) -> Self {
        let aspect = Self::calc_aspect_ratio(w as i32, h as i32);
        Self::init(aspect)
    }

    pub const fn attr(&self) -> CameraAttr {
        self.attr
    }

    pub fn attr_mut(&mut self) -> &mut CameraAttr {
        &mut self.attr
    }

    pub const fn view(&self) -> &Matrix {
        &self.view
    }

    pub fn projection(&self) -> &Matrix {
        self.projection.as_matrix()
    }

    pub const fn pos(&self) -> &Vector {
        &self.attr.eye
    }

    pub fn reset(&mut self) {
        let aspect = self.projection.aspect();
        *self = Self::init(aspect);
    }

    pub fn upt_aspect_ratio(&mut self, w: i32, h: i32) {
        let aspect = Self::calc_aspect_ratio(w, h);
        self.projection.set_aspect(aspect);
        self.upt();
    }

    pub fn upt_fov(&mut self, precise_y: f32) {
        self.attr.upt_fov(precise_y);
        self.projection.set_fovy(self.attr.fov * RADIAN);
        self.upt();
    }

    pub fn look_at(&mut self, xrel: i32, yrel: i32) {
        self.attr.look_at(xrel, yrel);
        self.upt();
    }

    pub fn input(&mut self, input: Flags) {
        self.attr.input(input);
        self.upt();
    }

    pub fn replace(&mut self, attr: CameraAttr) {
        self.attr = attr;
        self.upt();
    }

    pub fn upt(&mut self) {
        self.view = Matrix::look_at_rh(
            &self.attr.eye.into(),
            &(self.attr.eye + self.attr.target).into(),
            &self.attr.up,
        );
    }

    fn calc_aspect_ratio(w: i32, h: i32) -> f32 {
        w as f32 / h as f32
    }

    fn init(aspect: f32) -> Self {
        let attr = CameraAttr::default();
        let view = Matrix::identity();
        let projection = Perspective::new(aspect, attr.fov * RADIAN, 0.01, 1000.0);

        let mut cam = Self {
            attr,
            view,
            projection,
        };

        // initial setup
        cam.upt();

        cam
    }
}
