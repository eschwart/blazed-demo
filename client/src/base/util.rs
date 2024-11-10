use crate::*;

use std::ops::Deref;

use glow::{
    Context, HasContext, NativeProgram, BLEND, DEPTH_TEST, FRAGMENT_SHADER, LESS,
    ONE_MINUS_SRC_ALPHA, SRC_ALPHA, VERTEX_SHADER,
};
use sdl2::{
    video::{GLContext, Window},
    EventPump, EventSubsystem, Sdl, VideoSubsystem,
};

pub struct GL(glow::Context);

impl Deref for GL {
    type Target = glow::Context;

    fn deref(&self) -> &Self::Target {
        unsafe {
            let code = self.0.get_error();
            if code != 0 {
                error!("OPENGL: {}", code)
            }
        }
        &self.0
    }
}

pub fn init_shaders(gl: &Context) -> Result<NativeProgram> {
    let program = unsafe { gl.create_program()? };

    let shader_sources = [
        (VERTEX_SHADER, include_str!("../../shaders/shader.vert")),
        (FRAGMENT_SHADER, include_str!("../../shaders/shader.frag")),
    ];

    let mut shaders = Vec::with_capacity(shader_sources.len());

    for (shader_type, shader_source) in shader_sources {
        unsafe {
            let shader = gl.create_shader(shader_type)?;
            gl.shader_source(shader, shader_source);
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                panic!("{}", gl.get_shader_info_log(shader));
            }
            gl.attach_shader(program, shader);
            shaders.push(shader);
        }
    }

    unsafe {
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            panic!("{}", gl.get_program_info_log(program));
        }

        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }
    }
    Ok(program)
}

pub fn init() -> Result<(
    Sdl,
    VideoSubsystem,
    GL,
    Window,
    EventSubsystem,
    EventPump,
    GLContext,
)> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let gl_attr = video.gl_attr();
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(4, 6);
    let window = video
        .window("blazed", 1024, 769)
        .resizable()
        .opengl()
        .build()
        .map_err(Error::Window)?;
    let gl_context = window.gl_create_context()?;
    let gl = unsafe {
        glow::Context::from_loader_function(|s| video.gl_get_proc_address(s) as *const _)
    };
    let gl = GL(gl);
    let events = sdl.event()?;
    let event_pump = sdl.event_pump()?;

    unsafe {
        gl.enable(DEPTH_TEST);
        gl.depth_func(LESS);
        gl.depth_range_f32(0.1, 100.0);

        gl.enable(BLEND);
        gl.blend_func(SRC_ALPHA, ONE_MINUS_SRC_ALPHA);
    }

    Ok((sdl, video, gl, window, events, event_pump, gl_context))
}

pub fn free_object(gl: &Context, obj: &Object) {
    unsafe {
        gl.delete_vertex_array(obj.vao());
        gl.delete_buffer(obj.vbo());
        gl.delete_buffer(obj.ebo());
    }
}

pub fn free_objects<'a>(gl: &Context, objects: impl Iterator<Item = &'a Object>) {
    for obj in objects {
        free_object(gl, obj)
    }
}

pub fn clean_up<'a>(
    gl: &Context,
    program: NativeProgram,
    objects: impl Iterator<Item = &'a Object>,
) {
    unsafe {
        gl.delete_program(program);
    }
    free_objects(gl, objects);
}
