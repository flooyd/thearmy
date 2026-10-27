//! drawing thousands of circles in one go. macroquad draws a circle as a polygon of twenty triangles, which for an army of
//! thousands means hundreds of thousands of vertices made and sent every frame. here each circle is one small record and the
//! graphics card draws a square for every one, shaded to be round with a soft edge: the same few vertices are used for them all
//!
//! a disc with a nose is shaded as a small fighter instead (the units, when the setting says ships): the same square, the same record, and the shader
//! works out where the hull, the glass and the flame are for each pixel. a unit too small on screen to make out a ship is a circle, and the circles are
//! shaded exactly as they always were

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
    pub(crate) nose: [f32; 2], //[0, 0] for a circle, otherwise a vector pointing the way a ship is facing: it is drawn as a dart inside the same circle
}

impl Disc {
    pub(crate) fn new(x: f32, y: f32, radius: f32, color: Color) -> Disc {
        Disc { x, y, radius, thickness: 0.0, border: 0.0, color: to_bytes(color), nose: [0.0, 0.0] }
    }

    //a ring drawn the way draw_circle_lines does: the line is centred on the radius
    pub(crate) fn ring(x: f32, y: f32, radius: f32, thickness: f32, color: Color) -> Disc {
        Disc { x, y, radius: radius + thickness / 2.0, thickness, border: 0.0, color: to_bytes(color), nose: [0.0, 0.0] }
    }

    //the same disc with a black edge round it, which keeps it apart from the ones it is touching
    pub(crate) fn edged(self, border: f32) -> Disc {
        Disc { border, ..self }
    }

    //the same disc drawn as a ship pointing along a direction (a direction that is nothing leaves it a circle)
    pub(crate) fn pointing(self, nose: Vec2) -> Disc {
        let nose = nose.try_normalize().unwrap_or(Vec2::ZERO);
        Disc { nose: nose.to_array(), ..self }
    }

    pub(crate) fn is_ship(&self) -> bool {
        self.nose != [0.0, 0.0]
    }
}

// ---------- the ship ----------

//the one ship, a dart: a delta with a notch in its tail, drawn nose up (up is -y) in a box where the radius of a unit is SHIP_SPAN. the hull is the
//quadrilateral nose, wing, notch and the other wing; the cockpit is an ellipse on it and the engine a flame behind the notch. the shader gets these
//numbers pasted into it and the plain drawing reads them too, and the *_distance functions say what the shader works out so it can be tested
pub(crate) const SHIP_SPAN: f32 = 28.0;
pub(crate) const DART_NOSE: Vec2 = Vec2::new(0.0, -27.0);
pub(crate) const DART_WING: Vec2 = Vec2::new(19.5, 21.0);
const DART_NOTCH: Vec2 = Vec2::new(0.0, 12.5);
pub(crate) const COCKPIT_AT: Vec2 = Vec2::new(0.0, -5.5);
pub(crate) const COCKPIT_SIZE: Vec2 = Vec2::new(2.9, 6.5);
pub(crate) const FLAME_TIP: Vec2 = Vec2::new(0.0, 27.0);
pub(crate) const FLAME_BASE: Vec2 = Vec2::new(4.5, 15.0);
const FLAME_TOP: f32 = 15.0; //the flame is cut off flat here, at its base, just under the notch
pub(crate) const GLASS: [f32; 3] = [0.063, 0.125, 0.169];
pub(crate) const FLAME: [f32; 3] = [1.0, 0.65, 0.24];

//a ship has a black outline outside its hull, half as wide as a circle's border (that one is inside its radius), and never more than this many art units
const OUTLINE_SHARE: f32 = 0.5;
pub(crate) const OUTLINE_MAX: f32 = 3.5;
//the square a ship is drawn on reaches this far from the middle, in radiuses: past the wing tips and the outline
pub(crate) const SHIP_REACH: f32 = 1.2;
//a unit is a circle when it is smaller than this many pixels in radius, a ship above the second one, and in between it is a bit of both. a dart a few
//pixels across is only a dim triangle, and an army zoomed out should be as easy to see as ever
const SHIP_FROM: f32 = 4.0;
const SHIP_FULL: f32 = 8.0;

//the line through an edge as a normal pointing away from `inside` and how far the line is from the origin along it
fn edge_line(from: Vec2, to: Vec2, inside: Vec2) -> (Vec2, f32) {
    let along = (to - from).normalize();
    let mut normal = Vec2::new(along.y, -along.x);
    if normal.dot(inside - from) > 0.0 {
        normal = -normal;
    }
    (normal, normal.dot(from))
}

//the two lines that bound one half of the hull (the other half is its mirror): along the leading edge of the wing, and along its back
fn hull_lines() -> [(Vec2, f32); 2] {
    let inside = (DART_NOSE + DART_WING + DART_NOTCH) / 3.0;
    [edge_line(DART_NOSE, DART_WING, inside), edge_line(DART_WING, DART_NOTCH, inside)]
}

//and the line along the side of one half of the flame
fn flame_line() -> (Vec2, f32) {
    edge_line(FLAME_BASE, FLAME_TIP, Vec2::new(0.0, (FLAME_TIP.y + FLAME_BASE.y) / 2.0))
}

#[cfg(test)]
//how far a point is outside the hull, in the box of the art: negative inside it, and exact near the middle of an edge
pub(crate) fn hull_distance(point: Vec2) -> f32 {
    let point = Vec2::new(point.x.abs(), point.y);
    hull_lines().into_iter().map(|(normal, at)| normal.dot(point) - at).fold(f32::MIN, f32::max)
}

#[cfg(test)]
//the same for the cockpit
pub(crate) fn cockpit_distance(point: Vec2) -> f32 {
    ((point - COCKPIT_AT) / COCKPIT_SIZE).length() * COCKPIT_SIZE.x - COCKPIT_SIZE.x
}

#[cfg(test)]
//and for the flame
pub(crate) fn flame_distance(point: Vec2) -> f32 {
    let point = Vec2::new(point.x.abs(), point.y);
    let (normal, at) = flame_line();
    (normal.dot(point) - at).max(FLAME_TOP - point.y)
}

//a point in the box of the art as it lies in the world, for a ship at the middle of the screen pointing along `nose` with a radius of 1
pub(crate) fn art_to_world(art: Vec2, nose: Vec2) -> Vec2 {
    let right = Vec2::new(-nose.y, nose.x);
    (right * art.x - nose * art.y) / SHIP_SPAN
}

//the corners of the hull, in the box of the art: the nose, the right wing tip, the notch and the left wing tip
pub(crate) fn dart_corners() -> [Vec2; 4] {
    [DART_NOSE, DART_WING, DART_NOTCH, Vec2::new(-DART_WING.x, DART_WING.y)]
}

//the numbers of the art as the shader reads them
pub(crate) fn art_constants() -> String {
    let [(side, side_at), (tail, tail_at)] = hull_lines();
    let (flame, flame_at) = flame_line();
    let vec2 = |v: Vec2| format!("vec2({:?}, {:?})", v.x, v.y);
    let vec3 = |c: [f32; 3]| format!("vec3({:?}, {:?}, {:?})", c[0], c[1], c[2]);
    format!(
        "const float SPAN = {SHIP_SPAN:?};\nconst float REACH = {SHIP_REACH:?};\nconst float OUTLINE_SHARE = {OUTLINE_SHARE:?};\nconst float OUTLINE_MAX = {OUTLINE_MAX:?};\n\
         const float FROM = {SHIP_FROM:?};\nconst float FULL = {SHIP_FULL:?};\n\
         const vec2 SIDE = {};\nconst float SIDE_AT = {side_at:?};\nconst vec2 TAIL = {};\nconst float TAIL_AT = {tail_at:?};\n\
         const vec2 COCKPIT_AT = {};\nconst vec2 COCKPIT_SIZE = {};\nconst vec3 GLASS = {};\n\
         const vec2 FLAME_SIDE = {};\nconst float FLAME_AT = {flame_at:?};\nconst float FLAME_TOP = {FLAME_TOP:?};\nconst vec3 FLAME = {};\n",
        vec2(side),
        vec2(tail),
        vec2(COCKPIT_AT),
        vec2(COCKPIT_SIZE),
        vec3(GLASS),
        vec2(flame),
        vec3(FLAME),
    )
}

fn to_bytes(color: Color) -> [u8; 4] {
    [color.r, color.g, color.b, color.a].map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
}

//the units that are out in the open and in view (`view` already has a unit's radius to spare), as circles. a unit inside a
//tower is drawn by the tower, a champion by draw_champions, and the ones off screen would only cost the time to send them
#[cfg(test)]
pub(crate) fn unit_discs(units: &[Unit], view: Rect) -> Vec<Disc> {
    unit_discs_in(units, view, None, UnitShape::Circles, 1.0)
}

//the same, for a game with others in it: `team` is the colour of each player, which their army is tinted towards. `shape` is what a unit is drawn as: a
//circle with a black edge, or a ship that points the way it is going, which the shader draws inside the same square (and which is a circle again
//when it is too small on screen to make out). `scale` is how many times its size a unit is drawn (see figure_scale), and the ones that much bigger
//reach further into view from off it
pub(crate) fn unit_discs_in(units: &[Unit], view: Rect, team: Option<&[Color]>, shape: UnitShape, scale: f32) -> Vec<Disc> {
    let styled = |disc: Disc, unit: &Unit| match shape {
        UnitShape::Circles => disc.edged(UNIT_BORDER),
        UnitShape::Ships => disc.edged(UNIT_BORDER).pointing(unit.nose),
    };
    let spare = RADIUS * (scale - 1.0).max(0.0) * SHIP_REACH;
    let view = Rect::new(view.x - spare, view.y - spare, view.w + spare * 2.0, view.h + spare * 2.0);
    units
        .iter()
        .filter(|unit| unit.garrison.is_none() && unit.champion == 0 && view.contains(Vec2::new(unit.x, unit.y)))
        .map(|unit| {
            if unit.age >= UNIT_GROW_SECONDS {
                return styled(Disc::new(unit.x, unit.y, RADIUS * scale, unit_color_in(unit, team)), unit);
            }
            //just grown from a baby: it swells from a green speck, overshoots a little, and turns from green to its own colour
            let grown = (unit.age / UNIT_GROW_SECONDS).clamp(0.0, 1.0);
            let swell = 0.35 + 0.65 * (1.0 - (1.0 - grown).powi(3)) + 0.18 * (grown * std::f32::consts::PI).sin();
            styled(Disc::new(unit.x, unit.y, RADIUS * scale * swell, lerp_color(pickup_color(), unit_color_in(unit, team), grown)), unit)
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
    Bolts,
}

const LAYERS: usize = 6;

//the spare room a buffer is made with, so it only has to be made bigger now and then
const START_CAPACITY: usize = 4096;

//how many buffers each batch takes in turn: the card runs a frame or two behind, and filling a buffer it is still reading from
//makes the program wait for it
const BUFFERS: usize = 3;

//the shaders are these with the numbers of the art pasted in front of them (see art_constants)
pub(crate) const VERTEX: &str = r#"
attribute vec2 corner;
attribute vec4 disc;
attribute float border;
attribute vec4 tint;
attribute vec2 nose;

varying lowp vec4 color;
varying highp vec2 local;
varying highp vec2 art;
varying highp float edge;
varying highp float hole;
varying highp float rim;
varying highp float ship;
varying highp float outline;
varying highp float detail;

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
    //a disc with a nose is a ship once it is big enough on screen to make out, a circle when it is small, and a bit of both in between
    ship = dot(nose, nose) > 0.0 ? clamp((screen_radius - FROM) / (FULL - FROM), 0.0, 1.0) : 0.0;
    //a ship's outline goes outside its hull, as wide as half the border would be (in the units of the art), and the glass fades away when it is small
    outline = border > 0.0 ? min(OUTLINE_SHARE * width * SPAN / screen_radius, OUTLINE_MAX) : 0.0;
    detail = clamp((screen_radius - 5.0) / 7.0, 0.0, 1.0);
    //the square is a little bigger than the circle so the edge has room, and bigger still for a ship, which reaches out past the circle
    float reach = ship > 0.0 ? REACH : 1.0;
    local = corner * (reach + edge);
    //where the ship's art is under this corner: the nose is up in the art, so up is -y, and right is a quarter turn on from where the nose points
    art = vec2(dot(local, vec2(-nose.y, nose.x)), -dot(local, nose)) * SPAN;
    gl_Position = Projection * vec4(disc.xy + local * radius, 0.0, 1.0);
    color = tint / 255.0;
}"#;

pub(crate) const FRAGMENT: &str = r#"
varying lowp vec4 color;
varying highp vec2 local;
varying highp vec2 art;
varying highp float edge;
varying highp float hole;
varying highp float rim;
varying highp float ship;
varying highp float outline;
varying highp float detail;

//the pixel of a circle or a ring
vec4 circle_pixel() {
    float dist = length(local);
    float coverage = clamp((1.0 + 0.5 * edge - dist) / edge, 0.0, 1.0);
    if (hole >= 0.0) {
        coverage *= 1.0 - clamp((hole + 0.5 * edge - dist) / edge, 0.0, 1.0);
    }
    //inside the border it is the colour and outside it black, with the same soft edge between them as at the rim of the circle
    float inside = clamp((rim + 0.5 * edge - dist) / edge, 0.0, 1.0);
    return vec4(color.rgb * inside, color.a * coverage);
}

//the pixel of a ship. every shape has a soft edge a pixel wide, in art units (aa is how many of them fit across a pixel)
vec4 ship_pixel() {
    float aa = edge * SPAN;
    vec2 p = vec2(abs(art.x), art.y);
    float hull = max(dot(p, SIDE) - SIDE_AT, dot(p, TAIL) - TAIL_AT);
    float silhouette = clamp((outline + 0.5 * aa - hull) / aa, 0.0, 1.0);
    float body = clamp((0.5 * aa - hull) / aa, 0.0, 1.0);
    float cockpit = (length((p - COCKPIT_AT) / COCKPIT_SIZE) - 1.0) * COCKPIT_SIZE.x;
    float glass = detail * clamp((0.5 * aa - cockpit) / aa, 0.0, 1.0);
    float fire = clamp((0.5 * aa - max(dot(p, FLAME_SIDE) - FLAME_AT, FLAME_TOP - p.y)) / aa, 0.0, 1.0);
    //from the back: the flame, the black of the outline and hull over it, the colour of the hull over that
    float a = fire * (1.0 - silhouette) + silhouette;
    vec3 c = FLAME * fire * (1.0 - silhouette);
    c = c * (1.0 - body) + mix(color.rgb, GLASS, glass) * body;
    a = a * (1.0 - body) + body;
    return vec4(c / max(a, 0.0001), color.a * a);
}

void main() {
    if (ship <= 0.0) {
        gl_FragColor = circle_pixel();
    } else if (ship >= 1.0) {
        gl_FragColor = ship_pixel();
    } else {
        //turning from a circle into a ship, so they are blended with their colours weighted by how much of each there is
        vec4 round_one = circle_pixel();
        vec4 pointed_one = ship_pixel();
        float a = mix(round_one.a, pointed_one.a, ship);
        gl_FragColor = vec4(mix(round_one.rgb * round_one.a, pointed_one.rgb * pointed_one.a, ship) / max(a, 0.0001), a);
    }
}"#;

//the whole of each shader: its version, then (in the fragment shader, which has to say how precise its numbers are before it writes any) that, then the art,
//then the code that uses it
pub(crate) fn vertex_source() -> String {
    format!("#version 100\n{}{}", art_constants(), VERTEX)
}

pub(crate) fn fragment_source() -> String {
    format!("#version 100\nprecision highp float;\n{}{}", art_constants(), FRAGMENT)
}

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
        let (vertex, fragment) = (vertex_source(), fragment_source());
        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: &vertex, fragment: &fragment },
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
                VertexAttribute::with_buffer("nose", VertexFormat::Float2, 1),
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

//a ship drawn the slow way, with triangles: its black outline (the hull a little bigger), the hull, and the flame and the glass
fn draw_ship(disc: &Disc, color: Color) {
    let nose = Vec2::new(disc.nose[0], disc.nose[1]);
    let at = |art: Vec2| Vec2::new(disc.x, disc.y) + art_to_world(art, nose) * disc.radius;
    let [front, right, back, left] = dart_corners();
    let middle = (front + back) / 2.0;
    let swell = |corner: Vec2| middle + (corner - middle) * 1.12;
    let triangle = |a: Vec2, b: Vec2, c: Vec2, color: Color| draw_triangle(at(a), at(b), at(c), color);
    let flame = Color::new(FLAME[0], FLAME[1], FLAME[2], color.a);
    let black = Color::new(0.0, 0.0, 0.0, color.a);
    triangle(FLAME_BASE, Vec2::new(-FLAME_BASE.x, FLAME_BASE.y), FLAME_TIP, flame);
    triangle(swell(front), swell(right), swell(back), black);
    triangle(swell(front), swell(back), swell(left), black);
    triangle(front, right, back, color);
    triangle(front, back, left, color);
    let glass = at(COCKPIT_AT);
    draw_circle(glass.x, glass.y, COCKPIT_SIZE.x * disc.radius / SHIP_SPAN, Color::new(GLASS[0], GLASS[1], GLASS[2], color.a));
}

thread_local! {
    //None until the first circles are drawn, and None for good if this graphics card can't do it (they are drawn the slow way)
    static BATCHES: RefCell<[Option<Option<Batch>>; LAYERS]> = const { RefCell::new([None, None, None, None, None, None]) };
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
                if disc.is_ship() {
                    draw_ship(disc, color);
                } else if disc.border > 0.0 {
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
