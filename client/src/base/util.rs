use crate::*;
use enum_unit::*;
use glow::{
    Context, HasContext, NativeProgram, BLEND, CULL_FACE, DEPTH_TEST, FRAGMENT_SHADER, LESS,
    ONE_MINUS_SRC_ALPHA, SRC_ALPHA, VERTEX_SHADER,
};
use sdl2::{
    video::{GLContext, Window},
    EventPump, EventSubsystem, Sdl, VideoSubsystem,
};

#[derive(Clone, Copy, Debug, EnumUnit)]
pub enum Program {
    Simple(NativeProgram),
    Normal(NativeProgram),
}

impl Program {
    pub const fn native(&self) -> NativeProgram {
        match self {
            Self::Simple(program) => *program,
            Self::Normal(program) => *program,
        }
    }
}

#[derive(Debug)]
pub struct Shaders {
    simple: Program,
    normal: Program,
}

impl Shaders {
    pub const fn simple(&self) -> Program {
        self.simple
    }

    pub const fn normal(&self) -> Program {
        self.normal
    }

    pub fn delete(self, gl: &Context) {
        unsafe {
            gl.delete_program(self.simple.native());
            gl.delete_program(self.normal.native());
        }
    }
}

/// Production
#[cfg(not(debug_assertions))]
pub type GL = glow::Context;

/// Debugging
#[cfg(debug_assertions)]
pub struct GL(glow::Context);

#[cfg(debug_assertions)]
impl std::ops::Deref for GL {
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

pub fn process_shaders(gl: &Context, shader_sources: [(u32, &str); 2]) -> Result<NativeProgram> {
    let program = unsafe { gl.create_program()? };

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

pub fn init_shaders(gl: &Context) -> Result<Shaders> {
    let simple_shader_sources = [
        (
            VERTEX_SHADER,
            include_str!("../../shaders/simple/shader.vert"),
        ),
        (
            FRAGMENT_SHADER,
            include_str!("../../shaders/simple/shader.frag"),
        ),
    ];

    let normal_shader_sources = [
        (
            VERTEX_SHADER,
            include_str!("../../shaders/normal/shader.vert"),
        ),
        (
            FRAGMENT_SHADER,
            include_str!("../../shaders/normal/shader.frag"),
        ),
    ];

    let simple_shader = process_shaders(gl, simple_shader_sources)?;
    let normal_shader = process_shaders(gl, normal_shader_sources)?;

    let simple = Program::Simple(simple_shader);
    let normal = Program::Normal(normal_shader);

    let shaders = Shaders { simple, normal };
    Ok(shaders)
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

    let (width, height) = video.display_bounds(0)?.size();

    let window = video
        .window(
            "blazed",
            (width as f32 / 1.4) as u32,
            (height as f32 / 1.4) as u32,
        )
        .resizable()
        .opengl()
        .build()
        .map_err(Error::Window)?;

    let gl_context = window.gl_create_context()?;
    let gl = unsafe {
        glow::Context::from_loader_function(|s| video.gl_get_proc_address(s) as *const _)
    };

    #[cfg(debug_assertions)]
    let gl = GL(gl);

    let events = sdl.event()?;
    let event_pump = sdl.event_pump()?;

    unsafe {
        // TODO - implement some form of multisampling
        // gl.enable(MULTISAMPLE);

        // depth testing
        gl.enable(DEPTH_TEST);
        gl.depth_func(LESS);

        // face culling (default: back)
        gl.enable(CULL_FACE);

        // alpha transparency
        gl.enable(BLEND);
        gl.blend_func(SRC_ALPHA, ONE_MINUS_SRC_ALPHA);
    }

    Ok((sdl, video, gl, window, events, event_pump, gl_context))
}

pub fn free_buffers(gl: &Context, buffers: Buffers) {
    unsafe {
        gl.delete_vertex_array(buffers.vao());
        gl.delete_buffer(buffers.vbo());
        gl.delete_buffer(buffers.ebo());
    }
}

pub fn free_objects<'a>(gl: &Context, objects: impl Iterator<Item = &'a Object>) {
    objects.for_each(|obj| free_buffers(gl, obj.buffers()));
}

pub fn clean_up<'a>(gl: &Context, programs: Shaders, objects: impl Iterator<Item = &'a Object>) {
    programs.delete(gl);
    free_objects(gl, objects);
}
