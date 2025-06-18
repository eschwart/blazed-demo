use crate::*;
use bytemuck::{Pod, Zeroable};
use ultraviolet::Mat4;
use wopt::*;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Color {
    inner: [f32; 4],
    emits: u8,
}

impl Color {
    pub const fn new(inner: [f32; 4], emits: bool) -> Self {
        let emits = emits as u8; // cast to u8
        Self { inner, emits }
    }

    pub const fn is_emit(&self) -> bool {
        self.emits == 1
    }

    pub const fn alpha(&self) -> f32 {
        self.inner[3]
    }

    pub const fn is_opaque(alpha: f32) -> bool {
        alpha as i32 == 1
    }

    pub const fn data(&self) -> [f32; 4] {
        self.inner
    }
}

impl Default for Color {
    fn default() -> Self {
        Self {
            inner: [1.0; 4],
            emits: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, WithOpt)]
#[wopt(derive(Clone, Copy, Debug, Default))]
#[wopt(id = 12)]
pub struct Transformations {
    pub translation: Mat4,
    pub rotation: Mat4,
    pub scale: Mat4,
    pub model: Mat4,
}

impl Transformations {
    pub fn new(pos: Vec3, dim: Vec3) -> Self {
        let translation = Mat4::from_translation(pos);
        let rotation = Mat4::identity();
        let scale = Mat4::from_nonuniform_scale(dim);
        let model = Mat4::identity();

        Self {
            translation,
            rotation,
            scale,
            model,
        }
    }

    pub const fn translation(&self) -> Mat4 {
        self.translation
    }

    pub const fn rotation(&self) -> Mat4 {
        self.rotation
    }

    pub fn scale(&self) -> Mat4 {
        self.scale
    }

    pub fn scale_upt(&mut self, dim: Vec3) {
        self.scale = Mat4::from_nonuniform_scale(dim);
    }

    pub const fn model(&self) -> Mat4 {
        self.model
    }

    pub fn model_upt(&mut self) {
        let t = self.translation();
        let r = self.rotation();
        let s = self.scale();

        self.model = t * r * s;
    }
}

impl Default for Transformations {
    fn default() -> Self {
        let translation = Mat4::from_translation(Vec3::zero());
        let rotation = Mat4::identity();
        let scale = Mat4::from_nonuniform_scale(Vec3::one());
        let model = Mat4::identity();

        Self {
            translation,
            rotation,
            scale,
            model,
        }
    }
}
