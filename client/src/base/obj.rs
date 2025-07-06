use crate::*;
use glow::{
    Context, HasContext, NativeBuffer, NativeVertexArray, TRIANGLE_STRIP, TRIANGLES, UNSIGNED_BYTE,
};
use std::{collections::HashMap, fmt::Debug};
use ultraviolet::Vec3;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum InstanceKind {
    SimpleCube,
    NormalCube,
}

#[derive(Clone, Copy, Debug)]
pub struct ObjData {
    vao: NativeVertexArray,
    vbo: NativeBuffer,
    ebo: NativeBuffer,
    col_vbo: NativeBuffer,
    inst_vbo: NativeBuffer,
    mode: u32,
    element_type: u32,
    len: i32,
}

impl ObjData {
    pub const fn vao(&self) -> NativeVertexArray {
        self.vao
    }

    pub const fn vbo(&self) -> NativeBuffer {
        self.vbo
    }

    pub const fn ebo(&self) -> NativeBuffer {
        self.ebo
    }

    pub const fn col_vbo(&self) -> NativeBuffer {
        self.col_vbo
    }

    pub const fn inst_vbo(&self) -> NativeBuffer {
        self.inst_vbo
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

mod cube {
    use super::*;

    fn init<F>(
        gl: &glow::Context,
        vertices: &[f32],
        indices: &[u8],
        setup_vertex_attributes: F,
        primitive_mode: u32,
        first_instance_attrib: u32,
    ) -> Result<ObjData>
    where
        F: Fn(&glow::Context),
    {
        unsafe {
            let vao = gl.create_vertex_array()?;
            let vbo = gl.create_buffer()?;
            let ebo = gl.create_buffer()?;
            let col_vbo = gl.create_buffer()?;
            let inst_vbo = gl.create_buffer()?;

            gl.bind_vertex_array(Some(vao));

            // Vertex buffer
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(vertices),
                glow::STATIC_DRAW,
            );

            // Custom vertex attribute setup
            setup_vertex_attributes(gl);

            // Element buffer
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(indices),
                glow::STATIC_DRAW,
            );

            // Color buffer (per-instance)
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(col_vbo));
            gl.buffer_data_size(glow::ARRAY_BUFFER, 0, glow::DYNAMIC_DRAW);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, 4 * size_of::<f32>() as i32, 0);
            gl.vertex_attrib_divisor(1, 1);

            // Instance model matrices (per-instance)
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(inst_vbo));
            gl.buffer_data_size(glow::ARRAY_BUFFER, 0, glow::DYNAMIC_DRAW);
            let mat4_stride = 16 * size_of::<f32>() as i32;
            for i in 0..4 {
                let attrib_index = first_instance_attrib + i;
                gl.enable_vertex_attrib_array(attrib_index);
                gl.vertex_attrib_pointer_f32(
                    attrib_index,
                    4,
                    glow::FLOAT,
                    false,
                    mat4_stride,
                    (i * 16) as i32,
                );
                gl.vertex_attrib_divisor(attrib_index, 1);
            }

            // Cleanup
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);

            Ok(ObjData {
                vao,
                vbo,
                ebo,
                col_vbo,
                inst_vbo,
                mode: primitive_mode,
                element_type: UNSIGNED_BYTE,
                len: indices.len() as i32, // Consider removing this
            })
        }
    }

    /// A simple cube (8 vertices; 14 indices).
    ///
    /// Explanation: https://stackoverflow.com/a/79336923/13449866
    pub mod simple {
        use super::*;

        #[rustfmt::skip]
        pub const VERTICES: [f32; 24] = [
           1.0,  1.0, -1.0,  // [1, 1, 0] [00]
          -1.0,  1.0, -1.0,  // [0, 1, 0] [01]
           1.0,  1.0,  1.0,  // [1, 1, 1] [02]
          -1.0,  1.0,  1.0,  // [0, 1, 1] [03]
           1.0, -1.0, -1.0,  // [1, 0, 0] [04]
          -1.0, -1.0, -1.0,  // [0, 0, 0] [05]
          -1.0, -1.0,  1.0,  // [0, 0, 1] [06]
           1.0, -1.0,  1.0,  // [1, 0, 1] [07]

        ];

        #[rustfmt::skip]
        pub const INDICES: [u8; 14] = [
            0, 1, 4, 5, 6, 1, 3, 0, 2, 4, 7, 6, 2, 3
        ];

        pub fn init(gl: &glow::Context) -> Result<ObjData> {
            super::init(
                gl,
                &cube::simple::VERTICES,
                &cube::simple::INDICES,
                |gl| unsafe {
                    gl.enable_vertex_attrib_array(0);
                    gl.vertex_attrib_pointer_f32(
                        0,
                        3,
                        glow::FLOAT,
                        false,
                        3 * size_of::<f32>() as i32,
                        0,
                    );
                },
                TRIANGLE_STRIP,
                2,
            )
        }
    }

    /// Construct a normal cube (24 vertices; 36 indices).
    ///
    /// Explanation: https://stackoverflow.com/a/79337030/13449866
    pub mod normal {
        use super::*;

        #[rustfmt::skip]
        pub const VERTICES: [f32; 144]  = [
             // BACK
            -1.0,  -1.0,  -1.0,  /* [0, 0, 0] */   0.0,  0.0, -1.0,  //  [00]
            -1.0,   1.0,  -1.0,  /* [0, 1, 0] */   0.0,  0.0, -1.0,  //  [01]
             1.0,  -1.0,  -1.0,  /* [1, 0, 0] */   0.0,  0.0, -1.0,  //  [02]
             1.0,   1.0,  -1.0,  /* [1, 1, 0] */   0.0,  0.0, -1.0,  //  [03]

             // FRONT
            -1.0,  -1.0,   1.0,  /* [0, 0, 1] */   0.0,  0.0,  1.0,  //  [04]
            -1.0,   1.0,   1.0,  /* [0, 1, 1] */   0.0,  0.0,  1.0,  //  [05]
             1.0,  -1.0,   1.0,  /* [1, 0, 1] */   0.0,  0.0,  1.0,  //  [06]
             1.0,   1.0,   1.0,  /* [1, 1, 1] */   0.0,  0.0,  1.0,  //  [07]

             // LEFT
            -1.0,  -1.0,   1.0,  /* [0, 0, 1] */  -1.0,  0.0,  0.0,  //  [08]
            -1.0,   1.0,   1.0,  /* [0, 1, 1] */  -1.0,  0.0,  0.0,  //  [09]
            -1.0,  -1.0,  -1.0,  /* [0, 0, 0] */  -1.0,  0.0,  0.0,  //  [10]
            -1.0,   1.0,  -1.0,  /* [0, 1, 0] */  -1.0,  0.0,  0.0,  //  [11]

             // RIGHT
             1.0,  -1.0,   1.0,  /* [1, 0, 1] */  1.0,  0.0,  0.0,  //  [12]
             1.0,   1.0,   1.0,  /* [1, 1, 1] */  1.0,  0.0,  0.0,  //  [13]
             1.0,  -1.0,  -1.0,  /* [1, 0, 0] */  1.0,  0.0,  0.0,  //  [14]
             1.0,   1.0,  -1.0,  /* [1, 1, 0] */  1.0,  0.0,  0.0,  //  [15]

             // TOP
            -1.0,   1.0,  -1.0,  /* [0, 1, 0] */  0.0,  1.0,  0.0,  //  [16]
            -1.0,   1.0,   1.0,  /* [0, 1, 1] */  0.0,  1.0,  0.0,  //  [17]
             1.0,   1.0,  -1.0,  /* [1, 1, 0] */  0.0,  1.0,  0.0,  //  [18]
             1.0,   1.0,   1.0,  /* [1, 1, 1] */  0.0,  1.0,  0.0,  //  [19]

             // BOTTOM
            -1.0,  -1.0,  -1.0,  /* [0, 0, 0] */  0.0, -1.0,  0.0,  //  [20]
            -1.0,  -1.0,   1.0,  /* [0, 0, 1] */  0.0, -1.0,  0.0,  //  [21]
             1.0,  -1.0,  -1.0,  /* [1, 0, 0] */  0.0, -1.0,  0.0,  //  [22]
             1.0,  -1.0,   1.0,  /* [1, 0, 1] */  0.0, -1.0,  0.0,  //  [23]
        ];

        #[rustfmt::skip]
        pub const INDICES: [u8; 36] = [
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

        pub fn init(gl: &glow::Context) -> Result<ObjData> {
            super::init(
                gl,
                &cube::normal::VERTICES,
                &cube::normal::INDICES,
                |gl| unsafe {
                    // Position
                    gl.enable_vertex_attrib_array(0);
                    gl.vertex_attrib_pointer_f32(
                        0,
                        3,
                        glow::FLOAT,
                        false,
                        6 * size_of::<f32>() as i32,
                        0,
                    );
                    // Normal
                    gl.enable_vertex_attrib_array(2);
                    gl.vertex_attrib_pointer_f32(
                        2,
                        3,
                        glow::FLOAT,
                        false,
                        6 * size_of::<f32>() as i32,
                        3 * size_of::<f32>() as i32,
                    );
                },
                TRIANGLES,
                3,
            )
        }
    }
}

#[derive(Clone, Debug)]
pub struct InstanceData {
    pub trans: Transformations, // model matrix per instance
    pub color: Color,           // instance color, or any other per-instance attributes
}

#[derive(Clone, Debug)]
pub struct InstancedGroup {
    instances: Vec<InstanceData>, // all instance data for this group
    program: Program,
    data: ObjData,
}

impl InstancedGroup {
    pub const fn instances(&self) -> &[InstanceData] {
        self.instances.as_slice()
    }

    pub const fn program(&self) -> Program {
        self.program
    }

    pub const fn data(&self) -> &ObjData {
        &self.data
    }
}

impl InstancedGroup {
    pub const fn len(&self) -> usize {
        self.instances.len()
    }
}

#[derive(Debug)]
struct LookupInfo {
    kind: InstanceKind,
    idx: usize,
}

#[derive(Debug)]
pub struct RawObjects {
    groups: HashMap<InstanceKind, InstancedGroup>,
    lookup: HashMap<Id, LookupInfo>,
}

impl RawObjects {
    pub fn new(gl: &Context, shaders: &Shaders) -> Result<Self> {
        let mut groups = HashMap::default();

        groups.insert(
            InstanceKind::SimpleCube,
            InstancedGroup {
                instances: vec![],
                program: shaders.simple(),
                data: cube::simple::init(gl)?,
            },
        );

        groups.insert(
            InstanceKind::NormalCube,
            InstancedGroup {
                instances: vec![],
                program: shaders.normal(),
                data: cube::normal::init(gl)?,
            },
        );

        Ok(Self {
            groups,
            lookup: Default::default(),
        })
    }

    pub fn groups(&self) -> impl Iterator<Item = &InstancedGroup> {
        self.groups.values()
    }

    /// Create and add a new cube instance.
    pub fn new_cube(
        &mut self,
        id: Id,
        kind: InstanceKind,     // identifies the cube mesh/material group
        trans: Transformations, // model matrix
        color: [f32; 4],        // instance color
        emits: bool,            // light determinant
    ) {
        let color = Color::new(color, emits);

        let instance_data = InstanceData { trans, color };

        // get the group for this mesh/material
        let group = self.groups.get_mut(&kind).unwrap();

        // the to-be index of this `instance_data`
        let idx = group.instances.len();

        // add it to the system
        group.instances.push(instance_data);
        self.lookup.insert(id, LookupInfo { kind, idx });
    }

    /// Get immutable reference to instance data by object id
    pub fn get(&self, id: Id) -> Option<&InstanceData> {
        let LookupInfo { kind, idx } = self.lookup.get(&id)?;
        self.groups
            .get(kind)
            .and_then(|group| group.instances.get(*idx))
    }

    /// Get mutable reference to instance data by object id
    pub fn get_mut(&mut self, id: Id) -> Option<&mut InstanceData> {
        let LookupInfo { kind, idx } = self.lookup.get(&id)?;
        self.groups
            .get_mut(kind)
            .and_then(|group| group.instances.get_mut(*idx))
    }

    /// Remove an instance by object id
    pub fn remove(&mut self, id: Id) -> Option<InstanceData> {
        let LookupInfo { kind, idx } = self.lookup.remove(&id)?;
        let group = self.groups.get_mut(&kind)?;

        // TODO - use `swap_remove` instead (will need to update the swapped value's index in the lookup table).
        let removed = group.instances.remove(idx);

        Some(removed)
    }

    /// Retain only objects with a given kind (remove others)
    /// TODO - [maybe] introduce object types ([`ObjType`]`) into each instance data
    pub fn retain(&mut self, _kind: ObjType) {
        unimplemented!()
    }

    /// Iterator over all instance data in opaque groups
    pub fn opaque(&self) -> impl Iterator<Item = &InstanceData> {
        self.groups
            .values()
            .flat_map(|g| g.instances.iter().filter(|o| o.color.is_opaque()))
    }

    /// Iterator over all instance data in translucent groups
    pub fn translucent(&self) -> impl Iterator<Item = &InstanceData> {
        self.groups
            .values()
            .flat_map(|g| g.instances.iter().filter(|o| !o.color.is_opaque()))
    }

    /// Iterator over lights
    pub fn lights(&self) -> impl Iterator<Item = (Vec3, Vec3)> {
        self.groups().flat_map(|g| {
            g.instances.iter().filter_map(|o| {
                o.color
                    .is_emit() // only want objects emitting light
                    .then_some({
                        let pos = o.trans.translation.extract_translation();
                        let col = o.color.as_vec3(); // don't need alpha
                        (pos, col) // (POSITION, COLOR)
                    })
            })
        })
    }

    // Iterator over each instanced group's object buffers
    pub fn buffers(&self) -> impl Iterator<Item = &ObjData> {
        self.groups.values().map(InstancedGroup::data)
    }

    /// Total number of instances (opaque + translucent)
    pub fn len(&self) -> usize {
        self.groups.values().map(|g| g.instances.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
