//! drawing thousands of circles in one go. macroquad draws a circle as a polygon of twenty triangles, which for an army of
//! thousands means hundreds of thousands of vertices made and sent every frame. here each circle is one small record and the
//! graphics card draws a square for every one, shaded to be round with a soft edge: the same few vertices are used for them all

use crate::prelude::*;
use macroquad::miniquad::{
    Bindings, BlendFactor, BlendState, BlendValue, BufferId, BufferLayout, BufferSource, BufferType, BufferUsage, Equation, PassAction, Pipeline, PipelineParams, ShaderMeta, ShaderSource, UniformBlockLayout,
    UniformDesc, UniformType, UniformsSource, VertexAttribute, VertexFormat, VertexStep,
};
use std::cell::RefCell;

//one disc, or ring: where it is, how big and what colour. this is exactly what the graphics card reads for each one
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Disc {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) radius: f32,    //to the outside
    pub(crate) thickness: f32, //0 for a disc, otherwise how wide the ring is, inwards from the radius
    pub(crate) border: f32,    //0 for none, otherwise how wide a black edge is, inwards from the radius, as far as the zoom leaves room for it
    pub(crate) color: [u8; 4],
}

impl Disc {
    pub(crate) fn new(x: f32, y: f32, radius: f32, color: Color) -> Disc {
        Disc { x, y, radius, thickness: 0.0, border: 0.0, color: to_bytes(color) }
    }

    //a ring drawn the way draw_circle_lines does: the line is centred on the radius
    pub(crate) fn ring(x: f32, y: f32, radius: f32, thickness: f32, color: Color) -> Disc {
        Disc { x, y, radius: radius + thickness / 2.0, thickness, border: 0.0, color: to_bytes(color) }
    }

    //the same disc with a black edge round it, which keeps it apart from the ones it is touching
    pub(crate) fn edged(self, border: f32) -> Disc {
        Disc { border, ..self }
    }
}

fn to_bytes(color: Color) -> [u8; 4] {
    [color.r, color.g, color.b, color.a].map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
}

//the units that are out in the open and in view (`view` already has a unit's radius to spare), as circles. a unit inside a
//tower is drawn by the tower, a champion by draw_champions, and the ones off screen would only cost the time to send them
pub(crate) fn unit_discs(units: &[Unit], view: Rect) -> Vec<Disc> {
    units
        .iter()
        .filter(|unit| unit.garrison.is_none() && unit.champion == 0 && view.contains(Vec2::new(unit.x, unit.y)))
        .map(|unit| {
            if unit.age >= UNIT_GROW_SECONDS {
                return Disc::new(unit.x, unit.y, RADIUS, unit_color(unit)).edged(UNIT_BORDER);
            }
            //just grown from a baby: it swells from a green speck, overshoots a little, and turns from green to its own colour
            let grown = (unit.age / UNIT_GROW_SECONDS).clamp(0.0, 1.0);
            let swell = 0.35 + 0.65 * (1.0 - (1.0 - grown).powi(3)) + 0.18 * (grown * std::f32::consts::PI).sin();
            Disc::new(unit.x, unit.y, RADIUS * swell, lerp_color(pickup_color(), unit_color(unit), grown)).edged(UNIT_BORDER)
        })
        .collect()
}

//which batch of discs is being drawn. each has its own buffer on the graphics card, so filling one for the next frame
//never has to wait for the card to finish drawing the last
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Layer {
    Marks,
    Puffs,
    Army,
    Babies,
    Player,
}

const LAYERS: usize = 5;

//the spare room a buffer is made with, so it only has to be made bigger now and then
const START_CAPACITY: usize = 4096;

//how many buffers each batch takes in turn: the card runs a frame or two behind, and filling a buffer it is still reading from
//makes the program wait for it
const BUFFERS: usize = 3;

const VERTEX: &str = r#"#version 100
attribute vec2 corner;
attribute vec4 disc;
attribute float border;
attribute vec4 tint;

varying lowp vec4 color;
varying highp vec2 local;
varying highp float edge;
varying highp float hole;
varying highp float rim;

uniform mat4 Projection;
uniform float PixelsPerUnit;

void main() {
    float radius = disc.z;
    //the soft edge is a pixel wide, whatever the zoom
    edge = 1.0 / max(radius * PixelsPerUnit, 0.001);
    //a ring has a hole, this far out from the middle as a share of the radius, and a disc has none
    hole = disc.w > 0.0 ? max(radius - disc.w, 0.0) / radius : -1.0;
    //a border is as wide as asked on a big circle, at least a pixel, never more than a quarter of the radius, and fades away
    //on circles too small for the eye to make out an edge, which would only make them dark. rim is where the colour ends
    //and the black begins, as a share of the radius, and 2 for a circle with no border
    float screen_radius = radius * PixelsPerUnit;
    float width = min(max(border * PixelsPerUnit, 1.0), 0.28 * screen_radius) * clamp((screen_radius - 2.0) / 3.0, 0.0, 1.0);
    rim = (border > 0.0 && width > 0.05) ? 1.0 - width / screen_radius : 2.0;
    //the square is a little bigger than the circle so the edge has room
    local = corner * (1.0 + edge);
    gl_Position = Projection * vec4(disc.xy + local * radius, 0.0, 1.0);
    color = tint / 255.0;
}"#;

const FRAGMENT: &str = r#"#version 100
precision highp float;
varying lowp vec4 color;
varying highp vec2 local;
varying highp float edge;
varying highp float hole;
varying highp float rim;

void main() {
    float dist = length(local);
    float coverage = clamp((1.0 + 0.5 * edge - dist) / edge, 0.0, 1.0);
    if (hole >= 0.0) {
        coverage *= 1.0 - clamp((hole + 0.5 * edge - dist) / edge, 0.0, 1.0);
    }
    //inside the border it is the colour and outside it black, with the same soft edge between them as at the rim of the circle
    float inside = clamp((rim + 0.5 * edge - dist) / edge, 0.0, 1.0);
    gl_FragColor = vec4(color.rgb * inside, color.a * coverage);
}"#;

#[repr(C)]
struct Uniforms {
    projection: [f32; 16],
    pixels_per_unit: f32,
}

//what the graphics card needs to draw circles, made the first time it is asked to
struct Batch {
    pipeline: Pipeline,
    bindings: Bindings,
    capacity: usize,        //discs each of the buffers has room for
    buffers: Vec<BufferId>, //taken in turn, so the card can still be drawing from one while the next is filled
    next: usize,
}

impl Batch {
    fn new(ctx: &mut dyn macroquad::miniquad::RenderingBackend) -> Option<Batch> {
        if !ctx.info().features.instancing {
            return None;
        }
        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT },
                ShaderMeta {
                    images: Vec::new(),
                    uniforms: UniformBlockLayout { uniforms: vec![UniformDesc::new("Projection", UniformType::Mat4), UniformDesc::new("PixelsPerUnit", UniformType::Float1)] },
                },
            )
            .inspect_err(|error| eprintln!("discs: the shader would not compile ({error:?}), circles are drawn the slow way"))
            .ok()?;
        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default(), BufferLayout { stride: std::mem::size_of::<Disc>() as i32, step_func: VertexStep::PerInstance, step_rate: 1 }],
            &[
                VertexAttribute::with_buffer("corner", VertexFormat::Float2, 0),
                VertexAttribute::with_buffer("disc", VertexFormat::Float4, 1),
                VertexAttribute::with_buffer("border", VertexFormat::Float1, 1),
                VertexAttribute::with_buffer("tint", VertexFormat::Byte4, 1),
            ],
            shader,
            PipelineParams { color_blend: Some(BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::OneMinusValue(BlendValue::SourceAlpha))), ..Default::default() },
        );
        //the corners of the square every circle is drawn on, as two triangles
        let corners: [[f32; 2]; 4] = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
        let corner_buffer = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(&corners));
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let index_buffer = ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&indices));
        let buffers = Batch::disc_buffers(ctx, START_CAPACITY);
        Some(Batch { pipeline, bindings: Bindings { vertex_buffers: vec![corner_buffer, buffers[0]], index_buffer, images: Vec::new() }, capacity: START_CAPACITY, buffers, next: 0 })
    }

    fn disc_buffers(ctx: &mut dyn macroquad::miniquad::RenderingBackend, capacity: usize) -> Vec<BufferId> {
        (0..BUFFERS).map(|_| ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<Disc>(capacity))).collect()
    }
}

thread_local! {
    //None until the first circles are drawn, and None for good if this graphics card can't do it (they are drawn the slow way)
    static BATCHES: RefCell<[Option<Option<Batch>>; LAYERS]> = const { RefCell::new([None, None, None, None, None]) };
}

//draw discs in the world, over what has been drawn so far and under what is drawn next. `camera` is the one the world is
//being drawn with
pub(crate) fn draw_discs(layer: Layer, discs: &[Disc], camera: &Camera2D) {
    if discs.is_empty() {
        return;
    }
    BATCHES.with(|batches| {
        let mut batches = batches.borrow_mut();
        // SAFETY: macroquad's drawing is only done from this thread, and nothing it hands out is kept past this call
        let mut gl = unsafe { get_internal_gl() };
        let slot = &mut batches[layer as usize];
        let Some(batch) = slot.get_or_insert_with(|| Batch::new(gl.quad_context)).as_mut() else {
            for disc in discs {
                let color = Color::from_rgba(disc.color[0], disc.color[1], disc.color[2], disc.color[3]);
                if disc.border > 0.0 {
                    draw_circle(disc.x, disc.y, disc.radius, Color::new(0.0, 0.0, 0.0, color.a));
                    draw_circle(disc.x, disc.y, disc.radius - disc.border, color);
                } else if disc.thickness > 0.0 {
                    draw_circle_lines(disc.x, disc.y, disc.radius - disc.thickness / 2.0, disc.thickness, color);
                } else {
                    draw_circle(disc.x, disc.y, disc.radius, color);
                }
            }
            return;
        };
        //everything macroquad has been asked to draw so far is drawn first, so these go over it
        gl.flush();
        let ctx = &mut *gl.quad_context;
        if discs.len() > batch.capacity {
            for buffer in batch.buffers.drain(..) {
                ctx.delete_buffer(buffer);
            }
            batch.capacity = discs.len().next_power_of_two();
            batch.buffers = Batch::disc_buffers(ctx, batch.capacity);
        }
        batch.next = (batch.next + 1) % BUFFERS;
        batch.bindings.vertex_buffers[1] = batch.buffers[batch.next];
        ctx.buffer_update(batch.buffers[batch.next], BufferSource::slice(discs));
        let (width, height) = macroquad::miniquad::window::screen_size();
        let uniforms = Uniforms { projection: camera.matrix().to_cols_array(), pixels_per_unit: camera.zoom.x.abs() * width / 2.0 };
        ctx.begin_default_pass(PassAction::Nothing);
        ctx.apply_pipeline(&batch.pipeline);
        ctx.apply_viewport(0, 0, width as i32, height as i32);
        ctx.apply_scissor_rect(0, 0, width as i32, height as i32);
        ctx.apply_bindings(&batch.bindings);
        ctx.apply_uniforms(UniformsSource::table(&uniforms));
        ctx.draw(0, 6, discs.len() as i32);
        ctx.end_render_pass();
    });
}
