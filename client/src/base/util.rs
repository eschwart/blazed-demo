use crate::*;
use glow::{
    BLEND, CULL_FACE, Context, DEPTH_TEST, FRAGMENT_SHADER, HasContext, LESS, NativeProgram,
    NativeUniformLocation, ONE_MINUS_SRC_ALPHA, SRC_ALPHA, VERTEX_SHADER,
};
use sdl2::{
    EventPump, EventSubsystem, Sdl, VideoSubsystem,
    video::{GLContext, Window},
};

#[derive(Clone, Copy, Debug)]
pub enum ProgramKind {
    Simple,
    Normal,
}

#[derive(Clone, Copy, Debug)]
pub struct UnifLocs {
    view: Option<NativeUniformLocation>,
    proj: Option<NativeUniformLocation>,
    cam_pos: Option<NativeUniformLocation>,
    p_lights_len: Option<NativeUniformLocation>,
    p_lights: [[Option<NativeUniformLocation>; 2]; 16],
}

impl UnifLocs {
    pub fn new(gl: &Context, native: NativeProgram) -> Self {
        unsafe {
            let view = gl.get_uniform_location(native, "view");
            let proj = gl.get_uniform_location(native, "proj");
            let cam_pos = gl.get_uniform_location(native, "cam_pos");
            let p_lights_len = gl.get_uniform_location(native, "p_lights_len");

            let p_lights_dyn = (0..16)
                .map(|i| {
                    // set specific indexed light_pos[i]
                    let light_name = format!("p_lights[{i}]");
                    let light_pos_name = format!("{light_name}.pos");
                    let light_col_name = format!("{light_name}.col");

                    let pos_idx = gl.get_uniform_location(native, &light_pos_name);
                    let col_idx = gl.get_uniform_location(native, &light_col_name);

                    [pos_idx, col_idx]
                })
                .collect::<Vec<_>>();

            // SAFETY - currently always 16 (in the shader)
            let p_lights = p_lights_dyn.try_into().unwrap();

            Self {
                view,
                proj,
                cam_pos,
                p_lights_len,
                p_lights,
            }
        }
    }

    pub const fn view(&self) -> Option<&NativeUniformLocation> {
        self.view.as_ref()
    }

    pub const fn proj(&self) -> Option<&NativeUniformLocation> {
        self.proj.as_ref()
    }

    pub const fn cam_pos(&self) -> Option<&NativeUniformLocation> {
        self.cam_pos.as_ref()
    }

    pub const fn p_lights_len(&self) -> Option<&NativeUniformLocation> {
        self.p_lights_len.as_ref()
    }

    pub const fn p_lights(&self) -> &[[Option<NativeUniformLocation>; 2]] {
        self.p_lights.as_slice()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Program {
    native: NativeProgram,
    kind: ProgramKind,
    unif_locs: UnifLocs,
}

impl Program {
    pub fn simple(gl: &Context, native: NativeProgram) -> Self {
        Self {
            native,
            kind: ProgramKind::Simple,
            unif_locs: UnifLocs::new(gl, native),
        }
    }

    pub fn normal(gl: &Context, native: NativeProgram) -> Self {
        Self {
            native,
            kind: ProgramKind::Normal,
            unif_locs: UnifLocs::new(gl, native),
        }
    }

    pub const fn native(&self) -> NativeProgram {
        self.native
    }

    pub const fn kind(&self) -> ProgramKind {
        self.kind
    }

    pub fn unif_locs(&self) -> &UnifLocs {
        &self.unif_locs
    }
}

#[derive(Debug)]
pub struct Shaders {
    simple: Program,
    normal: Program,
    // add other shaders here (e.g.,
    // geometry: Program
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
                error!("OPENGL: {code}")
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

    let simple_prog = process_shaders(gl, simple_shader_sources)?;
    let normal_prog = process_shaders(gl, normal_shader_sources)?;

    let simple = Program::simple(gl, simple_prog);
    let normal = Program::normal(gl, normal_prog);

    let shaders = Shaders { simple, normal };
    Ok(shaders)
}

pub fn init() -> Result<(
    Sdl,
    VideoSubsystem,
    TimerSubsystem,
    GL,
    Window,
    EventSubsystem,
    EventPump,
    GLContext,
)> {
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let timer = sdl.timer()?;

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
        .position_centered()
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

    Ok((
        sdl, video, timer, gl, window, events, event_pump, gl_context,
    ))
}

pub fn free_buffers(gl: &Context, buffers: &ObjData) {
    unsafe {
        gl.delete_vertex_array(buffers.vao());
        gl.delete_buffer(buffers.vbo());
        gl.delete_buffer(buffers.ebo());
        gl.delete_buffer(buffers.col_vbo());
        gl.delete_buffer(buffers.inst_vbo());
    }
}

pub fn free_objects<'a>(gl: &Context, objects: impl Iterator<Item = &'a ObjData>) {
    objects.for_each(|obj| free_buffers(gl, obj));
}

pub fn clean_up<'a>(gl: &Context, programs: Shaders, objects: impl Iterator<Item = &'a ObjData>) {
    programs.delete(gl);
    free_objects(gl, objects);
}
