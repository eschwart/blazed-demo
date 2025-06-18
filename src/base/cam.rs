use crate::*;
use bytemuck::{Pod, Zeroable};
use std::ops::{AddAssign, SubAssign};
use ultraviolet::{Mat4, projection::perspective_gl};
use wopt::*;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct Axis([f32; 3]);

impl Axis {
    const X_BOUND: f32 = 360.0;
    const Y_LOWER_BOUND: f32 = -89.0;
    const Y_UPPER_BOUND: f32 = 89.0;

    pub const fn new(sensitivity: f32) -> Self {
        Self([0.0, 0.0, sensitivity])
    }

    pub const fn radians(&self) -> f32 {
        self.0[1]
    }

    pub const fn sensitivity(&self) -> f32 {
        self.0[2]
    }

    pub fn sensitivity_mut(&mut self) -> &mut f32 {
        &mut self.0[2]
    }

    fn x_bound(&mut self) {
        self.0[0] = (self.0[0] + Self::X_BOUND) % Self::X_BOUND;
    }

    fn y_bound(&mut self) {
        self.0[0] = clamp_unchecked(self.0[0], Self::Y_LOWER_BOUND, Self::Y_UPPER_BOUND);
    }

    fn update(&mut self) {
        self.0[1] = self.0[0].to_radians()
    }
}

impl AddAssign<i32> for Axis {
    fn add_assign(&mut self, rhs: i32) {
        self.0[0] += rhs as f32 * self.sensitivity();
    }
}

impl SubAssign<i32> for Axis {
    fn sub_assign(&mut self, rhs: i32) {
        self.0[0] -= rhs as f32 * self.sensitivity();
    }
}

#[derive(Clone, Copy, Debug, WithOpt)]
#[wopt(derive(Clone, Copy, Debug, Default))]
#[wopt(id = 10)]
pub struct CameraAttr {
    pub fov: f32,     // client's (player) field-of-vision
    pub speed: f32,   // player speed
    pub yaw: Axis,    // player's camera yaw   (used in rotation)
    pub pitch: Axis,  // player's camera pitch (used in rotation)
    pub eye: Vec3,    // player's position
    pub target: Vec3, // player's camera (target) vector
    pub up: Vec3,     // player's camera (up) vector
}

impl CameraAttr {
    pub fn new(pos: Vec3) -> Self {
        let mut attr = Self {
            fov: 90.0,
            speed: 0.024,
            yaw: Axis::new(0.052),
            pitch: Axis::new(0.050),
            eye: pos,
            target: Vec3::new(0.0, 0.0, -1.0),
            up: Vec3::new(0.0, 1.0, 0.0),
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
        let yaw_radians = self.yaw.radians();
        let pitch_radians = self.pitch.radians();

        let pitch_cos = pitch_radians.cos();

        // calculate new target values
        self.target.x = yaw_radians.sin() * pitch_cos;
        self.target.y = pitch_radians.sin();
        self.target.z = -yaw_radians.cos() * pitch_cos;

        // normalize
        self.target.normalize();
    }

    pub fn input(&mut self, kb: Keys) {
        let mut target = self.target;
        target.y = 0.0;

        for key in kb.iter() {
            match key {
                Keys::W => self.eye += target.normalized() * self.speed,
                Keys::A => self.eye -= target.cross(self.up).normalized() * self.speed,
                Keys::S => self.eye -= target.normalized() * self.speed,
                Keys::D => self.eye += target.cross(self.up).normalized() * self.speed,

                Keys::SPACE => self.eye += self.up * self.speed,
                Keys::SHIFT => self.eye -= self.up * self.speed,

                _ => (),
            }
        }
    }
}

impl Default for CameraAttr {
    fn default() -> Self {
        Self::new(Vec3::zero())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RawCamera {
    attr: CameraAttr,
    view: Mat4,
    projection: Mat4,
    aspect_ratio: f32,
}

impl RawCamera {
    pub fn new((w, h): (u32, u32)) -> Self {
        let aspect = Self::calc_aspect_ratio(w as i32, h as i32);
        Self::init(aspect)
    }

    pub const fn attr(&self) -> &CameraAttr {
        &self.attr
    }

    pub fn attr_mut(&mut self) -> &mut CameraAttr {
        &mut self.attr
    }

    pub const fn view(&self) -> &Mat4 {
        &self.view
    }

    pub const fn projection(&self) -> &Mat4 {
        &self.projection
    }

    pub const fn pos(&self) -> &Vec3 {
        &self.attr.eye
    }

    pub fn reset(&mut self) {
        *self = Self::init(self.aspect_ratio);
    }

    pub fn upt_aspect_ratio(&mut self, w: i32, h: i32) {
        let aspect_ratio = Self::calc_aspect_ratio(w, h);
        self.projection = perspective_gl(self.attr.fov * RADIAN, aspect_ratio, 0.01, 1000.0);
        self.upt();
    }

    pub fn upt_fov(&mut self, precise_y: f32) {
        self.attr.upt_fov(precise_y);
        self.projection = perspective_gl(self.attr.fov * RADIAN, self.aspect_ratio, 0.01, 1000.0);
        self.upt();
    }

    pub fn look_at(&mut self, xrel: i32, yrel: i32) {
        self.attr.look_at(xrel, yrel);
        self.upt();
    }

    pub fn input(&mut self, kb: Keys) {
        self.attr.input(kb);
        self.upt();
    }

    pub fn replace(&mut self, attr: CameraAttr) {
        self.attr = attr;
        self.upt();
    }

    pub fn upt(&mut self) {
        self.view = Mat4::look_at(
            self.attr.eye,
            self.attr.eye + self.attr.target,
            self.attr.up,
        );
    }

    fn calc_aspect_ratio(w: i32, h: i32) -> f32 {
        w as f32 / h as f32
    }

    fn init(aspect_ratio: f32) -> Self {
        let attr = CameraAttr::default();
        let view = Mat4::identity();
        let projection = perspective_gl(attr.fov * RADIAN, aspect_ratio, 0.01, 1000.0);

        let mut cam = Self {
            attr,
            view,
            projection,
            aspect_ratio,
        };

        // initial setup
        cam.upt();

        cam
    }
}
