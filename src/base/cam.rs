use crate::*;

use nalgebra::{Matrix4, Perspective3, Point3, Rotation3, Translation3, UnitQuaternion, Vector3};
use serde::{Deserialize, Serialize};

const RADIAN: f32 = std::f32::consts::PI / 180.0;

pub type Point = Point3<f32>;
pub type Vector = Vector3<f32>;
pub type Matrix = Matrix4<f32>;
pub type Translation = Translation3<f32>;
pub type Rotation = Rotation3<f32>;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CameraAttr {
    pub fov: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub speed: f32,
    pub sensitivity: f32,
    pub eye: Vector,
    pub target: Vector,
    pub up: Vector,
}

impl CameraAttr {
    pub const fn new() -> Self {
        Self {
            fov: 90.0,
            yaw: 0.0,
            pitch: 0.0,
            speed: 0.05,
            sensitivity: 0.05,
            eye: Vector::new(0.0, 0.0, 0.0),
            target: Vector::new(0.0, 0.0, -1.0),
            up: Vector::new(0.0, 1.0, 0.0),
        }
    }

    pub fn upt_fov(&mut self, precise_y: f32) {
        self.fov -= precise_y;
        self.fov = clamp_unchecked(self.fov, 30.0, 110.0);
    }

    pub fn look_at(&mut self, xrel: i32, yrel: i32) {
        self.yaw -= xrel as f32 * self.sensitivity;
        self.pitch += yrel as f32 * self.sensitivity;

        self.pitch = clamp_unchecked(self.pitch, -89.0, 89.0);

        // is this necessary?
        if self.yaw <= -360.0 || self.yaw >= 360.0 {
            self.yaw = 0.0
        }

        let yaw_radians = self.yaw * RADIAN;
        let pitch_radians = self.pitch * RADIAN;

        let pitch_cos = pitch_radians.cos();

        self.target.x = yaw_radians.sin() * pitch_cos;
        self.target.y = pitch_radians.sin();
        self.target.z = -yaw_radians.cos() * pitch_cos;

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

                Flags::UP => (),
                Flags::LEFT => (),
                Flags::DOWN => (),
                Flags::RIGHT => (),

                Flags::SPACE => self.eye += self.up * self.speed,
                Flags::SHIFT => self.eye -= self.up * self.speed,
                Flags::CTRL => (),

                _ => error!("Unexpected keyboard input"),
            }
        }
    }

    pub fn rotation(&self) -> Matrix {
        let rotation_yaw = UnitQuaternion::from_axis_angle(&Vector::y_axis(), -self.yaw * RADIAN);
        let rotation_pitch =
            UnitQuaternion::from_axis_angle(&Vector::x_axis(), self.pitch * RADIAN);
        (rotation_yaw * rotation_pitch).to_homogeneous()
    }

    pub fn translation(&self) -> Matrix {
        Translation::from(self.eye).to_homogeneous()
    }
}

impl Default for CameraAttr {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, PartialEq)]
pub struct Camera {
    pub attr: CameraAttr,
    pub model: Matrix,
    pub view: Matrix,
    pub projection: Perspective3<f32>,
}

impl Camera {
    pub fn init((w, h): (u32, u32)) -> Self {
        let attr = CameraAttr::default();
        let aspect = Self::calc_aspect_ratio(w as i32, h as i32);
        let fov = attr.fov;

        Self {
            attr,
            model: Matrix4::identity(),
            view: Matrix4::identity(),
            projection: Perspective3::new(aspect, fov * RADIAN, 0.01, 1000.0),
        }
    }

    fn calc_aspect_ratio(w: i32, h: i32) -> f32 {
        w as f32 / h as f32
    }

    pub fn model(&self) -> &[f32] {
        self.model.as_slice()
    }

    pub fn view(&self) -> &[f32] {
        self.view.as_slice()
    }

    pub fn projection(&self) -> &[f32] {
        self.projection.as_matrix().as_slice()
    }

    pub fn near(&self) -> f32 {
        self.projection.znear()
    }

    pub fn far(&self) -> f32 {
        self.projection.zfar()
    }

    pub fn upt_aspect_ratio(&mut self, w: i32, h: i32) {
        self.projection.set_aspect(Self::calc_aspect_ratio(w, h));
        self.upt()
    }

    pub fn upt_fov(&mut self, precise_y: f32) {
        self.attr.upt_fov(precise_y);
        self.projection.set_fovy(self.attr.fov * RADIAN);
        self.upt()
    }

    pub fn look_at(&mut self, xrel: i32, yrel: i32) {
        self.attr.look_at(xrel, yrel);
        self.upt();
    }

    pub fn input(&mut self, input: Flags) {
        self.attr.input(input);
        self.upt();
    }

    pub fn upt(&mut self) {
        self.view = Matrix4::look_at_rh(
            &self.attr.eye.into(),
            &(self.attr.eye + self.attr.target).into(),
            &self.attr.up,
        );
    }
}
