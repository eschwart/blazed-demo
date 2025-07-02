use crate::*;
use bytemuck::{NoUninit, cast_slice};
use glow::{
    ARRAY_BUFFER, Context, ELEMENT_ARRAY_BUFFER, FLOAT, HasContext, NativeBuffer,
    NativeVertexArray, STATIC_DRAW, TRIANGLE_STRIP, TRIANGLES, UNSIGNED_BYTE,
};
use ordered_float::OrderedFloat;
use std::{
    collections::HashMap,
    fmt::Debug,
    ops::{Deref, DerefMut},
};
use ultraviolet::{Mat4, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct Buffers {
    vao: NativeVertexArray,
    vbo: NativeBuffer,
    ebo: NativeBuffer,
}

impl Buffers {
    pub const fn vao(&self) -> NativeVertexArray {
        self.vao
    }

    pub const fn vbo(&self) -> NativeBuffer {
        self.vbo
    }

    pub const fn ebo(&self) -> NativeBuffer {
        self.ebo
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Object {
    program: Program,
    buffers: Buffers,
    mode: u32,
    element_type: u32,
    len: i32,
    data: UptObj,
    tran: Transformations,
}

impl Object {
    fn new(
        program: Program,
        buffers: Buffers,
        mode: u32,
        element_type: u32,
        len: i32,
        data: UptObj,
    ) -> Self {
        let mut tran = Transformations::new(data.cam.eye, data.dim);
        tran.model_upt(); // initial transformation

        Self {
            program,
            buffers,
            data,
            mode,
            element_type,
            len,
            tran,
        }
    }

    pub fn tran(&self) -> &Transformations {
        &self.tran
    }

    pub fn tran_mut(&mut self) -> &mut Transformations {
        &mut self.tran
    }

    pub fn translation_upt(&mut self, pos: Vec3) {
        self.tran.translation = Mat4::from_translation(pos)
    }

    pub fn rotation_upt(&mut self, pitch: f32, yaw: f32) {
        self.tran.rotation = Mat4::from_euler_angles(0.0, pitch, yaw)
    }

    pub fn transform_upt(&mut self) {
        self.translation_upt(self.cam.eye);

        let yaw = self.cam.yaw;
        let pitch = self.cam.pitch;

        self.rotation_upt(pitch.radians(), -yaw.radians());
        self.tran.scale_upt(self.dim);
        self.tran.model_upt();
    }

    /// Construct a simple cube (8 vertices; 14 indices).
    ///
    /// Explanation: https://stackoverflow.com/a/79336923/13449866
    pub fn create_flat_cube(
        gl: &Context,
        program: Program,
        id: Id,
        kind: ObjType,
        pos: Vec3,
        dim: Vec3,
        color: Color,
    ) -> Result<Self> {
        let cam = CameraAttr::new(pos);
        Self::create_flat_cube_with(
            gl,
            program,
            UptObj {
                id,
                kind,
                dim,
                color,
                cam,
                ..Default::default()
            },
        )
    }

    /// Construct a simple cube (8 vertices; 14 indices) with specified [`ObjectData`].
    ///
    /// Explanation: https://stackoverflow.com/a/79336923/13449866
    pub fn create_flat_cube_with(gl: &Context, program: Program, data: UptObj) -> Result<Self> {
        let x = -1.0;
        let y = -1.0;
        let z = -1.0;

        let xw = 1.0;
        let yh = 1.0;
        let zd = 1.0;

        #[rustfmt::skip]
        let vertices = [
           xw,  yh,   z,  // [1, 1, 0] [00]
            x,  yh,   z,  // [0, 1, 0] [01]
           xw,  yh,  zd,  // [1, 1, 1] [02]
            x,  yh,  zd,  // [0, 1, 1] [03]
           xw,   y,   z,  // [1, 0, 0] [04]
            x,   y,   z,  // [0, 0, 0] [05]
            x,   y,  zd,  // [0, 0, 1] [06]
           xw,   y,  zd,  // [1, 0, 1] [07]
        ];

        #[rustfmt::skip]
        let indices = [
            0, 1, 4, 5, 6, 1, 3, 0, 2, 4, 7, 6, 2, 3
        ];

        Self::from_raw::<f32, u8>(
            gl,
            program,
            vertices.as_slice(),
            indices.as_slice(),
            TRIANGLE_STRIP,
            UNSIGNED_BYTE,
            data,
            false,
        )
    }

    /// Construct a normal cube (24 vertices; 36 indices).
    ///
    /// Explanation: https://stackoverflow.com/a/79337030/13449866
    pub fn create_cube(
        gl: &Context,
        program: Program,
        id: Id,
        kind: ObjType,
        pos: Vec3,
        dim: Vec3,
        color: Color,
    ) -> Result<Self> {
        let cam = CameraAttr::new(pos);
        let data = UptObj {
            id,
            kind,
            dim,
            color,
            cam,
            ..Default::default()
        };

        match program.kind() {
            ProgramUnit::Simple => Self::create_flat_cube_with(gl, program, data),
            ProgramUnit::Normal => Self::create_cube_with(gl, program, data),
            _ => unreachable!(),
        }
    }

    /// Construct a normal cube (24 vertices; 36 indices) with specified [`ObjectData`].
    ///
    /// Explanation: https://stackoverflow.com/a/79337030/13449866
    pub fn create_cube_with(gl: &Context, program: Program, data: UptObj) -> Result<Self> {
        let x = -1.0;
        let y = -1.0;
        let z = -1.0;

        let xw = 1.0;
        let yh = 1.0;
        let zd = 1.0;

        #[rustfmt::skip]
        let vertices = [
             // BACK
             x,   y,   z,  /* [0, 0, 0] */   0.0,  0.0, -1.0,  //  [00]
             x,  yh,   z,  /* [0, 1, 0] */   0.0,  0.0, -1.0,  //  [01]
            xw,   y,   z,  /* [1, 0, 0] */   0.0,  0.0, -1.0,  //  [02]
            xw,  yh,   z,  /* [1, 1, 0] */   0.0,  0.0, -1.0,  //  [03]

             // FRONT
             x,   y,  zd,  /* [0, 0, 1] */   0.0,  0.0,  1.0,  //  [04]
             x,  yh,  zd,  /* [0, 1, 1] */   0.0,  0.0,  1.0,  //  [05]
            xw,   y,  zd,  /* [1, 0, 1] */   0.0,  0.0,  1.0,  //  [06]
            xw,  yh,  zd,  /* [1, 1, 1] */   0.0,  0.0,  1.0,  //  [07]

             // LEFT
             x,   y,  zd,  /* [0, 0, 1] */  -1.0,  0.0,  0.0,  //  [08]
             x,  yh,  zd,  /* [0, 1, 1] */  -1.0,  0.0,  0.0,  //  [09]
             x,   y,   z,  /* [0, 0, 0] */  -1.0,  0.0,  0.0,  //  [10]
             x,  yh,   z,  /* [0, 1, 0] */  -1.0,  0.0,  0.0,  //  [11]

             // RIGHT
             xw,   y,  zd,  /* [1, 0, 1] */  1.0,  0.0,  0.0,  //  [12]
             xw,  yh,  zd,  /* [1, 1, 1] */  1.0,  0.0,  0.0,  //  [13]
             xw,   y,   z,  /* [1, 0, 0] */  1.0,  0.0,  0.0,  //  [14]
             xw,  yh,   z,  /* [1, 1, 0] */  1.0,  0.0,  0.0,  //  [15]

             // TOP
              x,  yh,   z,  /* [0, 1, 0] */  0.0,  1.0,  0.0,  //  [16]
              x,  yh,  zd,  /* [0, 1, 1] */  0.0,  1.0,  0.0,  //  [17]
             xw,  yh,   z,  /* [1, 1, 0] */  0.0,  1.0,  0.0,  //  [18]
             xw,  yh,  zd,  /* [1, 1, 1] */  0.0,  1.0,  0.0,  //  [19]

             // BOTTOM
              x,   y,   z,  /* [0, 0, 0] */  0.0, -1.0,  0.0,  //  [20]
              x,   y,  zd,  /* [0, 0, 1] */  0.0, -1.0,  0.0,  //  [21]
             xw,   y,   z,  /* [1, 0, 0] */  0.0, -1.0,  0.0,  //  [22]
             xw,   y,  zd,  /* [1, 0, 1] */  0.0, -1.0,  0.0,  //  [23]
        ];

        #[rustfmt::skip]
        let indices = [
            // FRONT
             0,  3,  2,    1,  3,  0,

            // BACK
             6,  7,  4,    4,  7,  5,

            // LEFT
             8, 11, 10,    9, 11,  8,

            // RIGHT
            14, 15, 12,   12, 15, 13,

            // TOP
            16, 19, 18,   17, 19, 16,

            // BOTTOM
            22, 23, 20,   20, 23, 21,
        ];

        Self::from_raw::<f32, u8>(
            gl,
            program,
            vertices.as_slice(),
            indices.as_slice(),
            TRIANGLES,
            UNSIGNED_BYTE,
            data,
            true,
        )
    }

    pub fn from_raw<V: NoUninit, I: NoUninit>(
        gl: &Context,
        program: Program,
        vertices: &[V],
        indices: &[I],
        mode: u32,
        element_type: u32,
        data: UptObj,
        has_norms: bool,
    ) -> Result<Self> {
        unsafe {
            // creates and bind Vertex Array Object (VAO)
            let vao = gl.create_vertex_array()?;
            let vbo = gl.create_buffer()?;
            let ebo = gl.create_buffer()?;

            let mut stride = 3;

            if has_norms {
                stride += 3
            }

            gl.bind_vertex_array(Some(vao));

            // create and bind Vertex Buffer Object (VBO)
            gl.bind_buffer(ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, cast_slice(vertices), STATIC_DRAW);

            // create and bind Elements Buffer Object (EBO)
            gl.bind_buffer(ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(ELEMENT_ARRAY_BUFFER, cast_slice(indices), STATIC_DRAW);

            // enable `pos` attribute
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, FLOAT, false, stride * size_of::<f32>() as i32, 0);

            if has_norms {
                // enable `norm` attribute
                gl.enable_vertex_attrib_array(1);
                gl.vertex_attrib_pointer_f32(
                    1,
                    3,
                    FLOAT,
                    false,
                    stride * size_of::<f32>() as i32,
                    3 * size_of::<f32>() as i32,
                );
            }

            // unbind buffers
            gl.bind_vertex_array(None);
            gl.bind_buffer(ARRAY_BUFFER, None);
            gl.bind_buffer(ELEMENT_ARRAY_BUFFER, None);

            let buf = Buffers { vao, vbo, ebo };

            Ok(Self::new(
                program,
                buf,
                mode,
                element_type,
                indices.len() as i32,
                data,
            ))
        }
    }

    pub const fn program(&self) -> Program {
        self.program
    }

    pub const fn buffers(&self) -> Buffers {
        self.buffers
    }

    pub const fn data(&self) -> &UptObj {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut UptObj {
        &mut self.data
    }

    pub const fn vao(&self) -> NativeVertexArray {
        self.buffers.vao()
    }

    pub const fn vbo(&self) -> NativeBuffer {
        self.buffers.vbo()
    }

    pub const fn ebo(&self) -> NativeBuffer {
        self.buffers.ebo()
    }

    pub const fn id(&self) -> Id {
        self.data.id
    }

    pub const fn color(&self) -> [f32; 4] {
        self.data.color.data()
    }

    pub const fn mode(&self) -> u32 {
        self.mode
    }

    pub const fn element_type(&self) -> u32 {
        self.element_type
    }

    pub const fn len(&self) -> i32 {
        self.len
    }
}

impl Deref for Object {
    type Target = UptObj;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for Object {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[derive(Clone, Debug, Default)]
struct ObjInfo {
    index: usize,
    is_opaque: bool,
}

#[derive(Clone, Debug, Default)]
pub struct RawObjects {
    opaque: Vec<Object>,
    translucent: Vec<Object>,
    lookup: HashMap<Id, ObjInfo>,

    lights: HashMap<Id, Object>,
}

impl RawObjects {
    /// create and add a new cube with specified attributes.
    pub fn new_cube(
        &mut self,
        gl: &Context,
        program: Program,
        id: Id,
        kind: ObjType,
        pos: Vec3,
        dim: Vec3,
        color: Color,
    ) -> Result {
        let cam = CameraAttr::new(pos);
        let data = UptObj {
            id,
            kind,
            dim,
            color,
            cam,
            ..Default::default()
        };
        self.new_cube_with(gl, program, data)
    }

    /// create and add a new cube with specified [`ObjectData`].
    pub fn new_cube_with(&mut self, gl: &Context, program: Program, data: UptObj) -> Result {
        let obj = Object::create_cube_with(gl, program, data)?;
        self.insert(obj);
        Ok(())
    }

    /// return a mutable reference of the specified object.
    pub fn get_mut(&mut self, id: Id) -> Option<&mut Object> {
        let &ObjInfo { index, is_opaque } = self.lookup.get(&id)?;
        if is_opaque {
            self.opaque.get_mut(index)
        } else {
            self.translucent.get_mut(index)
        }
    }

    /// insert a new object.
    pub fn insert(&mut self, obj: Object) {
        let is_opaque = obj.color.alpha() == 1.0;
        let index = if is_opaque {
            let index = self.opaque.len();
            self.opaque.push(obj);
            index
        } else {
            let index = self.opaque.len();
            self.translucent.push(obj);
            index
        };
        self.lookup.insert(obj.id(), ObjInfo { index, is_opaque });
    }

    /// remove the object specified object.
    pub fn remove(&mut self, id: Id) -> Option<Object> {
        let ObjInfo { index, is_opaque } = self.lookup.remove(&id)?;

        Some(if is_opaque {
            self.opaque.remove(index)
        } else {
            self.translucent.remove(index)
        })
    }

    /// retain only the objects specified by object type.
    pub fn retain(&mut self, gl: &Context, kind: ObjType) {
        self.opaque.retain(|obj| {
            if kind == obj.kind {
                free_buffers(gl, obj.buffers());
                false
            } else {
                true
            }
        });
        self.translucent.retain(|obj| {
            if kind == obj.kind {
                free_buffers(gl, obj.buffers());
                false
            } else {
                true
            }
        });
        self.lights.retain(|_, obj| {
            if kind == obj.kind {
                free_buffers(gl, obj.buffers());
                false
            } else {
                true
            }
        });
    }

    /// create and add a new light (simple shading with color as light color) object.
    pub fn new_light(
        &mut self,
        gl: &Context,
        program: Program,
        id: Id,
        kind: ObjType,
        pos: Vec3,
        dim: Vec3,
        color: Color,
    ) -> Result {
        let obj = Object::create_flat_cube(gl, program, id, kind, pos, dim, color)?;
        self.lights.insert(obj.id(), obj);
        Ok(())
    }

    /// return an iterator of every opaque object.
    pub fn opaque(&self) -> impl Iterator<Item = &Object> {
        self.opaque.iter()
    }

    /// return an iterator of every translucent object.
    pub fn translucent(&self) -> impl Iterator<Item = &Object> {
        self.translucent.iter()
    }

    /// return an iterator of every light object.
    pub fn lights(&self) -> impl Iterator<Item = &Object> {
        self.lights.values()
    }

    /// return an iterator of every translucent object, sorted from furthest to closest.
    pub fn translucent_sorted(&self, pos: Vec3) -> impl Iterator<Item = &Object> {
        let mut sorted: Vec<&Object> = self.translucent.iter().collect();
        sorted.sort_by_cached_key(|o| OrderedFloat(-(pos - o.cam.eye).mag_sq()));
        sorted.into_iter()
    }

    /// return an iterator over every object (generally used for cleanup).
    pub fn iter(&self) -> impl Iterator<Item = &Object> {
        self.opaque().chain(self.translucent()).chain(self.lights())
    }
}
