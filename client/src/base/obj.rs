use crate::*;

use std::fmt::Debug;

use bytemuck::{cast_slice, Pod};
use glow::{
    Context, HasContext, NativeBuffer, NativeVertexArray, Program, ARRAY_BUFFER,
    ELEMENT_ARRAY_BUFFER, FLOAT, STATIC_DRAW,
};

#[derive(Debug)]
struct Buffers {
    vao: NativeVertexArray,
    vbo: NativeBuffer,
    ebo: NativeBuffer,
}

#[derive(Debug)]
pub struct Object {
    program: Program,
    buf: Buffers,
    color: [f32; 4],
    len: i32,
    id: u8,
}

impl Object {
    const fn new(program: Program, buf: Buffers, color: [f32; 4], len: i32, id: u8) -> Self {
        Self {
            program,
            buf,
            color,
            len,
            id,
        }
    }

    pub fn from_obj<V: Pod + Debug>(
        gl: &Context,
        program: Program,
        o: Obj<V>,
        color: [f32; 4],
        id: u8,
    ) -> Result<Self> {
        let vertices = o.vertices;
        let indices = o.indices;

        unsafe {
            // creates and bind Vertex Array Object (VAO)
            let vao = gl.create_vertex_array()?;
            let vbo = gl.create_buffer()?;
            let ebo = gl.create_buffer()?;

            gl.bind_vertex_array(Some(vao));

            // create and bind Vertex Buffer Object (VBO)
            gl.bind_buffer(ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(ARRAY_BUFFER, cast_slice(&vertices), STATIC_DRAW);

            // create and bind Elements Buffer Object (EBO)
            gl.bind_buffer(ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(ELEMENT_ARRAY_BUFFER, cast_u16_slice(&indices), STATIC_DRAW);

            // enable `pos` attribute
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, FLOAT, false, 6 * size_of::<f32>() as i32, 0);

            // enable `pos` attribute
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(
                1,
                3,
                FLOAT,
                false,
                6 * size_of::<f32>() as i32,
                3 * size_of::<f32>() as i32,
            );

            // unbind buffers
            gl.bind_vertex_array(None);
            gl.bind_buffer(ARRAY_BUFFER, None);
            gl.bind_buffer(ELEMENT_ARRAY_BUFFER, None);

            let buf = Buffers { vao, vbo, ebo };
            let len = indices.len();

            Ok(Self::new(program, buf, color, len as i32, id))
        }
    }

    pub const fn program(&self) -> Program {
        self.program
    }

    pub const fn vao(&self) -> NativeVertexArray {
        self.buf.vao
    }

    pub const fn vbo(&self) -> NativeBuffer {
        self.buf.vbo
    }

    pub const fn ebo(&self) -> NativeBuffer {
        self.buf.ebo
    }

    pub const fn color(&self) -> &[f32] {
        self.color.as_slice()
    }

    pub const fn len(&self) -> i32 {
        self.len
    }

    pub const fn id(&self) -> u8 {
        self.id
    }
}
