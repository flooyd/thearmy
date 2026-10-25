//! the enemies, the bosses and the marks put on them, drawn as the creatures they are: each is a drawing made of shapes (see art/), which the graphics card works
//! out for every pixel of a square the creature is on, the way it works out a ship (see discs.rs). a creature is one small record, and all of one kind are drawn in
//! one go. there is a program for each kind, since one that did them all would be too big for a graphics card to make in a reasonable time, and each is only made when
//! the first one of its kind is drawn
//!
//! the drawings face up and reach a little past the hitbox, which is the middle of the picture: the square of the picture is PICTURE times as wide

use crate::creature_art::{Art, ARTS, HELPERS};
use crate::prelude::*;
use macroquad::miniquad::{
    Bindings, BlendFactor, BlendState, BlendValue, BufferId, BufferLayout, BufferSource, BufferType, BufferUsage, Equation, PassAction, Pipeline, PipelineParams, ShaderMeta, ShaderSource, UniformBlockLayout,
    UniformDesc, UniformType, UniformsSource, VertexAttribute, VertexFormat, VertexStep,
};
use std::cell::RefCell;
use std::sync::OnceLock;

//how many art units there are from the middle of the picture to its edge (the hitbox is the middle 100 of the 160), and so how much wider than the hitbox the picture is
pub(crate) const SPAN: f32 = 80.0;
pub(crate) const PICTURE: f32 = 1.6;

//the spare room a buffer is made with, and how many there are of them for each kind, taken in turn so that the card can still be drawing from one while the next is
//filled (see discs.rs)
const START_CAPACITY: usize = 64;
const BUFFERS: usize = 3;

//one creature as the graphics card reads it
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Instance {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) radius: f32,     //from the middle to the edge of the square it is drawn on
    pub(crate) seed: f32,       //seconds added to the time, so that what spins on one does not spin in step with the next
    pub(crate) facing: [f32; 2], //a unit vector: the way the top of the drawing points
    pub(crate) color: [u8; 4],  //the colour of its body, and how much of it is seen (the alpha)
    pub(crate) hurt: f32,       //1 for the look of a boss in its last stand
}

//a creature to draw: which of the drawings (the number of it in ARTS, from 1) and where and how big and which way round and what colour
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Creature {
    pub(crate) shape: usize,
    pub(crate) instance: Instance,
}

impl Creature {
    //`size` is the side of the hitbox in pixels. one that does not turn (a round one, a nest) is drawn upright whichever way it is facing
    pub(crate) fn new(shape: usize, middle: Vec2, size: f32, facing: Vec2, color: Color) -> Creature {
        let facing = if ARTS[shape - 1].turns { facing.try_normalize().unwrap_or(Vec2::new(0.0, -1.0)) } else { Vec2::new(0.0, -1.0) };
        let bytes = [color.r, color.g, color.b, color.a].map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8);
        Creature { shape, instance: Instance { x: middle.x, y: middle.y, radius: size / 2.0 * PICTURE, seed: 0.0, facing: facing.to_array(), color: bytes, hurt: 0.0 } }
    }

    //the same with its seed, which is what makes what spins on it not spin in step with the others
    pub(crate) fn seeded(self, seed: u32) -> Creature {
        Creature { instance: Instance { seed: (seed % 997) as f32 * 0.37, ..self.instance }, ..self }
    }

    //the same in the look of a boss in its last stand, or not
    pub(crate) fn hurt(self, hurt: bool) -> Creature {
        Creature { instance: Instance { hurt: if hurt { 1.0 } else { 0.0 }, ..self.instance }, ..self }
    }
}

//the number of the drawing with this name in the game, from 1
pub(crate) fn shape_named(name: &str) -> Option<usize> {
    static BY_NAME: OnceLock<HashMap<&'static str, usize>> = OnceLock::new();
    BY_NAME.get_or_init(|| ARTS.iter().enumerate().map(|(i, art)| (art.name, i + 1)).collect()).get(name).copied()
}

//the order the kinds are drawn in: the creatures from the smallest drawing to the biggest, so that a big one is over a small one it is standing on, and then the
//marks, which go over the lot
pub(crate) fn draw_order() -> &'static [usize] {
    static ORDER: OnceLock<Vec<usize>> = OnceLock::new();
    ORDER.get_or_init(|| {
        let mut order: Vec<usize> = (0..ARTS.len()).collect();
        order.sort_by(|&a, &b| (ARTS[a].mark, ARTS[a].size).partial_cmp(&(ARTS[b].mark, ARTS[b].size)).unwrap_or(std::cmp::Ordering::Equal));
        order
    })
}

//the creatures sorted into a list for each kind, in the order they are given (`lists` has a list for each of the drawings, and is left empty by what draws them)
pub(crate) fn sort_into(creatures: &[Creature], lists: &mut [Vec<Instance>]) {
    for creature in creatures {
        lists[creature.shape - 1].push(creature.instance);
    }
}

//the part of the world a camera shows
pub(crate) fn camera_view(camera: &Camera2D) -> Rect {
    let (half_width, half_height) = (1.0 / camera.zoom.x.abs().max(1.0e-9), 1.0 / camera.zoom.y.abs().max(1.0e-9));
    Rect::new(camera.target.x - half_width, camera.target.y - half_height, half_width * 2.0, half_height * 2.0)
}

// ---------- the shaders ----------

const VERTEX: &str = r#"
attribute vec2 corner;
attribute vec4 place;
attribute vec2 facing;
attribute vec4 tint;
attribute float hurt;

varying lowp vec4 color;
varying highp vec2 spot;
varying highp float aa;
varying highp float look;
varying highp float seed;

uniform mat4 Projection;
uniform float PixelsPerUnit;

void main() {
    float radius = place.z;
    //half of the picture is this many pixels across, and so a pixel is this many art units
    float screen = max(radius * PixelsPerUnit, 0.001);
    aa = SPAN / screen;
    //the square is a pixel bigger on every side than the picture, so that its soft edge has room
    vec2 local = corner * (1.0 + 1.0 / screen);
    //where the drawing is under this corner: the top of it is up (-y) and points the way it is facing, and right is a quarter turn on from there
    spot = vec2(dot(local, vec2(-facing.y, facing.x)), -dot(local, facing)) * SPAN;
    gl_Position = Projection * vec4(place.xy + local * radius, 0.0, 1.0);
    color = tint / 255.0;
    look = hurt;
    seed = place.w;
}"#;

const FRAGMENT: &str = r#"
uniform float Time;

varying lowp vec4 color;
varying highp vec2 spot;
varying highp float aa;
varying highp float look;
varying highp float seed;

void main() {
    //the hitbox is the middle 100 units, and when that is only a few pixels across there is nothing to see of the drawing: it is a square in the colour of the creature
    //(and a mark, which is only an extra, is not seen at all)
    float detail = clamp((50.0 / aa - 3.0) / 6.0, 0.0, 1.0);
    vec4 c = vec4(0.0);
    if (detail > 0.0) {
        c = art(spot, aa, color.rgb, look, Time + seed) * detail;
    }
    if (detail < 1.0 && PLAIN > 0.5) {
        vec4 square = vec4(0.0);
        float edge = min(max(2.0, aa), 14.0);
        paint(square, max(abs(spot.x), abs(spot.y)) - (50.0 - edge), edge, aa, color.rgb);
        c += square * (1.0 - detail);
    }
    //(the colour has its alpha multiplied in, so what is drawn over what was there is one minus that of it)
    gl_FragColor = c * color.a;
}"#;

//the whole of the vertex shader
pub(crate) fn creature_vertex() -> String {
    format!("#version 100\nconst float SPAN = {SPAN:?};\n{VERTEX}")
}

//the whole of the fragment shader of one drawing: the shapes, then the drawing, then what is done with it
pub(crate) fn creature_fragment(art: &Art) -> String {
    format!("#version 100\nprecision highp float;\nconst float SPAN = {SPAN:?};\nconst float PLAIN = {:?};\n{HELPERS}\n{}\n{FRAGMENT}", if art.mark { 0.0 } else { 1.0 }, art.glsl)
}

#[repr(C)]
struct Uniforms {
    projection: [f32; 16],
    pixels_per_unit: f32,
    time: f32,
}

// ---------- drawing them ----------

//what the graphics card needs to draw one kind
struct Kind {
    pipeline: Pipeline,
    buffers: Vec<BufferId>, //taken in turn
    capacity: usize,
    next: usize,
}

//what is shared by all of them, made the first time any is drawn. `None` for good if this graphics card can't do it (they are squares)
struct Engine {
    corners: BufferId,
    indices: BufferId,
    kinds: Vec<Option<Option<Kind>>>, //(None: not made yet, Some(None): it would not compile)
    lists: Vec<Vec<Instance>>,
}

impl Engine {
    fn new(ctx: &mut dyn macroquad::miniquad::RenderingBackend) -> Option<Engine> {
        if !ctx.info().features.instancing {
            return None;
        }
        let corners: [[f32; 2]; 4] = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        Some(Engine {
            corners: ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(&corners)),
            indices: ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&indices)),
            kinds: (0..ARTS.len()).map(|_| None).collect(),
            lists: (0..ARTS.len()).map(|_| Vec::new()).collect(),
        })
    }
}

impl Kind {
    fn new(ctx: &mut dyn macroquad::miniquad::RenderingBackend, art: &Art) -> Option<Kind> {
        let (vertex, fragment) = (creature_vertex(), creature_fragment(art));
        let started = Instant::now();
        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: &vertex, fragment: &fragment },
                ShaderMeta {
                    images: Vec::new(),
                    uniforms: UniformBlockLayout {
                        uniforms: vec![UniformDesc::new("Projection", UniformType::Mat4), UniformDesc::new("PixelsPerUnit", UniformType::Float1), UniformDesc::new("Time", UniformType::Float1)],
                    },
                },
            )
            .inspect_err(|error| eprintln!("creatures: the shader of {} would not compile ({error:?}), it is drawn as a square", art.name))
            .ok()?;
        if std::env::var_os("ARMY_TIMING").is_some() {
            eprintln!("creatures: {} compiled in {:.0} ms", art.name, started.elapsed().as_secs_f32() * 1000.0);
        }
        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default(), BufferLayout { stride: std::mem::size_of::<Instance>() as i32, step_func: VertexStep::PerInstance, step_rate: 1 }],
            &[
                VertexAttribute::with_buffer("corner", VertexFormat::Float2, 0),
                VertexAttribute::with_buffer("place", VertexFormat::Float4, 1),
                VertexAttribute::with_buffer("facing", VertexFormat::Float2, 1),
                VertexAttribute::with_buffer("tint", VertexFormat::Byte4, 1),
                VertexAttribute::with_buffer("hurt", VertexFormat::Float1, 1),
            ],
            shader,
            //(what is drawn has its alpha multiplied in already)
            PipelineParams { color_blend: Some(BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::OneMinusValue(BlendValue::SourceAlpha))), ..Default::default() },
        );
        Some(Kind { pipeline, buffers: Kind::buffers(ctx, START_CAPACITY), capacity: START_CAPACITY, next: 0 })
    }

    fn buffers(ctx: &mut dyn macroquad::miniquad::RenderingBackend, capacity: usize) -> Vec<BufferId> {
        (0..BUFFERS).map(|_| ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<Instance>(capacity))).collect()
    }
}

thread_local! {
    //None until the first creature is drawn
    static ENGINE: RefCell<Option<Option<Engine>>> = const { RefCell::new(None) };
}

//make the program of every kind now, which takes a moment (a second or two on a graphics card that has not made them before), rather than when the first of a kind is
//drawn, which would be a pause in the middle of a game, the first time a boss turns up. a kind that can't be made is a square, as it would be later
pub(crate) fn warm_up() {
    ENGINE.with(|engine| {
        let mut engine = engine.borrow_mut();
        // SAFETY: macroquad's drawing is only done from this thread, and nothing it hands out is kept past this call
        let gl = unsafe { get_internal_gl() };
        let Some(engine) = engine.get_or_insert_with(|| Engine::new(gl.quad_context)).as_mut() else {
            return;
        };
        for (kind, art) in engine.kinds.iter_mut().zip(ARTS.iter()) {
            kind.get_or_insert_with(|| Kind::new(&mut *gl.quad_context, art));
        }
    });
}

//a creature drawn the slow way, as a square in its colour: what there is when the graphics card can't do better
fn draw_square(instance: &Instance) {
    let half = instance.radius / PICTURE;
    let color = Color::from_rgba(instance.color[0], instance.color[1], instance.color[2], instance.color[3]);
    draw_rectangle(instance.x - half, instance.y - half, half * 2.0, half * 2.0, color);
}

//draw creatures in the world, over what has been drawn so far and under what is drawn next. `camera` is the one the world is being drawn with
pub(crate) fn draw_creatures(creatures: &[Creature], camera: &Camera2D) {
    if creatures.is_empty() {
        return;
    }
    ENGINE.with(|engine| {
        let mut engine = engine.borrow_mut();
        // SAFETY: macroquad's drawing is only done from this thread, and nothing it hands out is kept past this call
        let mut gl = unsafe { get_internal_gl() };
        let Some(engine) = engine.get_or_insert_with(|| Engine::new(gl.quad_context)).as_mut() else {
            for creature in creatures.iter().filter(|creature| !ARTS[creature.shape - 1].mark) {
                draw_square(&creature.instance);
            }
            return;
        };
        sort_into(creatures, &mut engine.lists);
        //everything macroquad has been asked to draw so far is drawn first, so these go over it
        gl.flush();
        let ctx = &mut *gl.quad_context;
        let (width, height) = macroquad::miniquad::window::screen_size();
        let uniforms = Uniforms { projection: camera.matrix().to_cols_array(), pixels_per_unit: camera.zoom.x.abs() * width / 2.0, time: (get_time() % 7200.0) as f32 };
        //what is needed is made before the pass: a kind is made when the first of it is drawn, and its buffers made bigger when there are more than they hold. one that
        //would not compile is drawn as squares (if it is not a mark), after all the others
        let mut squares = Vec::new();
        for &which in draw_order() {
            if engine.lists[which].is_empty() {
                continue;
            }
            match engine.kinds[which].get_or_insert_with(|| Kind::new(ctx, &ARTS[which])).as_mut() {
                None => {
                    if !ARTS[which].mark {
                        squares.extend(engine.lists[which].iter().copied());
                    }
                    engine.lists[which].clear();
                }
                Some(kind) => {
                    let wanted = engine.lists[which].len();
                    if wanted > kind.capacity {
                        for buffer in kind.buffers.drain(..) {
                            ctx.delete_buffer(buffer);
                        }
                        kind.capacity = wanted.next_power_of_two();
                        kind.buffers = Kind::buffers(ctx, kind.capacity);
                    }
                }
            }
        }
        ctx.begin_default_pass(PassAction::Nothing);
        for &which in draw_order() {
            let list = std::mem::take(&mut engine.lists[which]);
            let Some(Some(kind)) = engine.kinds[which].as_mut().filter(|_| !list.is_empty()) else {
                engine.lists[which] = list;
                continue;
            };
            kind.next = (kind.next + 1) % BUFFERS;
            let bindings = Bindings { vertex_buffers: vec![engine.corners, kind.buffers[kind.next]], index_buffer: engine.indices, images: Vec::new() };
            ctx.buffer_update(kind.buffers[kind.next], BufferSource::slice(&list));
            ctx.apply_pipeline(&kind.pipeline);
            ctx.apply_viewport(0, 0, width as i32, height as i32);
            ctx.apply_scissor_rect(0, 0, width as i32, height as i32);
            ctx.apply_bindings(&bindings);
            ctx.apply_uniforms(UniformsSource::table(&uniforms));
            ctx.draw(0, 6, list.len() as i32);
            engine.lists[which] = list;
            engine.lists[which].clear();
        }
        ctx.end_render_pass();
        for instance in &squares {
            draw_square(instance);
        }
    });
}

// ---------- which way they face ----------

//which way a creature is facing, and where it was when that last changed (so that the way it is going can be seen, a bit at a time)
#[derive(Clone, Copy, Debug)]
pub(crate) struct Heading {
    pub(crate) facing: Vec2,
    going: Vec2, //the way it was last seen going
    anchor: Vec2,
}

//how far a creature has to have gone to be seen going somewhere, in pixels
const MOVED_ENOUGH: f32 = 2.0;
//and how far is further than anything walks in a frame: it was put somewhere else
const JUMPED: f32 = 400.0;

impl Heading {
    //a creature that has not been seen before, facing `want` if there is somewhere it is facing, and if there is not a way that is made from its number
    pub(crate) fn new(at: Vec2, id: u32, want: Option<Vec2>) -> Heading {
        let facing = want.and_then(|want| want.try_normalize()).unwrap_or_else(|| Vec2::from_angle(id as f32 * 2.399_963));
        Heading { facing, going: facing, anchor: at }
    }

    //turn it for a frame, at `rate` radians a second at the most: towards `want` if there is somewhere it has to face, and if there is not towards the way it
    //has been going (and if it has not gone anywhere, it is left as it was). returns which way it faces now
    pub(crate) fn turn(&mut self, at: Vec2, want: Option<Vec2>, rate: f32, dt: f32) -> Vec2 {
        let moved = at - self.anchor;
        if moved.length() > JUMPED {
            self.anchor = at;
        } else if moved.length() >= MOVED_ENOUGH {
            self.anchor = at;
            self.going = moved / moved.length();
        }
        let target = want.and_then(|want| want.try_normalize()).unwrap_or(self.going);
        let angle = self.facing.perp_dot(target).atan2(self.facing.dot(target));
        let step = angle.clamp(-rate * dt, rate * dt);
        self.facing = Vec2::from_angle(step).rotate(self.facing).try_normalize().unwrap_or(target);
        self.facing
    }
}
