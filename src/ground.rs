//! the ground the world stands on: a meadow of grass, drawn by the graphics card in one go over the part of the world in view. the blades come in three
//! sizes and each zoom shows the ones that come out about a pixel or more wide, fading from one size to the next, so it is grass all the way out. gusts of
//! wind roll across it, bending the blades and lighting them, and big sunny and shaded patches and bare spots keep it from looking tiled. it is the look
//! the owner chose from the mock-up ("Deep meadow"): dark, so the army, the warnings and the babies stand out on it. a card that can't run the shader
//! keeps the flat floor (see `draw_world`)

use crate::prelude::*;
use macroquad::miniquad::{
    Bindings, BufferLayout, BufferSource, BufferType, BufferUsage, PassAction, Pipeline, PipelineParams, ShaderMeta, ShaderSource, UniformBlockLayout, UniformDesc, UniformType, UniformsSource,
    VertexAttribute, VertexFormat,
};
use std::cell::RefCell;

//the three sizes of blade: the cells of the ground each grows three in, in world pixels, from the biggest
pub(crate) const GRASS_CELLS: [f32; 3] = [115.0, 38.0, 14.0];
//a blade's width, on average, as a share of its cell
const BLADE_SHARE: f32 = 0.81 * 0.15;

const VERTEX: &str = r#"#version 100
attribute vec2 corner;

varying highp vec2 world;

uniform mat4 Projection;
uniform vec4 Area;

void main() {
    world = mix(Area.xy, Area.zw, corner);
    gl_Position = Projection * vec4(world, 0.0, 1.0);
}"#;

const FRAGMENT: &str = r#"#version 100
precision highp float;

varying highp vec2 world;

uniform float PixelsPerUnit;
uniform float Time;
uniform float Wind;
uniform vec3 Fade;

//Deep meadow: the turf, the blades low and high, their tips, and the sheen of a blade the wind bends
const vec3 SOIL = vec3(0.10, 0.085, 0.055);
const vec3 LOW = vec3(0.050, 0.140, 0.060);
const vec3 HIGH = vec3(0.150, 0.350, 0.115);
const vec3 TIP = vec3(0.430, 0.590, 0.210);
const vec3 SHEEN = vec3(0.32, 0.38, 0.16);

float patchy;

float hash12(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

vec3 hash32(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * vec3(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yxz + 33.33);
    return fract((p3.xxy + p3.yzz) * p3.zyx);
}

float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash12(i), hash12(i + vec2(1.0, 0.0)), u.x), mix(hash12(i + vec2(0.0, 1.0)), hash12(i + vec2(1.0, 1.0)), u.x), u.y);
}

float fbm(vec2 p) {
    float sum = 0.0;
    float amp = 0.5;
    for (int octave = 0; octave < 4; octave++) {
        sum += amp * noise(p);
        p = p * 2.03 + vec2(17.1, 9.3);
        amp *= 0.5;
    }
    return sum / 0.9375;
}

//how hard the wind gusts at a place: long fronts across the wind, rolling to the right
float gust(vec2 w) {
    vec2 q = w * 0.0011;
    q.x -= Time * 0.16;
    float g = fbm(q * vec2(1.4, 0.55) + vec2(0.0, Time * 0.02));
    return smoothstep(0.48, 0.80, g) * Wind;
}

//the turf under the blades: sunny and shaded patches, bare spots, and the sheen of the gusts
vec3 turf(vec2 w, float g) {
    patchy = smoothstep(0.25, 0.80, fbm(w * 0.0021));
    vec3 c = mix(LOW * 0.50, LOW * 1.05, patchy);
    float bare = smoothstep(0.64, 0.80, fbm(w * 0.0047 + 13.0));
    c = mix(c, SOIL, bare * 0.6);
    c *= 0.9 + 0.2 * noise(w * 0.011);
    return c + SHEEN * g * 0.16;
}

//one size of blade: three to a cell, leaning up the screen (the figures are seen a little from the south) and bent by the gust. a blade reaches a
//cell up or to either side, so the cell the pixel is in and the row below it are looked at, the row below last as it is in front
vec3 blades(vec3 col, vec2 w, float cell, float salt, float fade, float g) {
    vec2 c = floor(w / cell);
    float aa = 1.0 / PixelsPerUnit;
    for (int j = 0; j < 2; j++) {
        for (int i = -1; i < 2; i++) {
            vec2 id = c + vec2(float(i), float(j));
            for (int k = 0; k < 3; k++) {
                vec3 h = hash32(id + vec2(float(k) * 0.37 + salt, salt * 1.7));
                vec3 e = hash32(id.yx + vec2(salt * 2.3, float(k) * 0.61 + 5.0));
                vec2 root = (id + h.xy) * cell;
                float len = cell * (0.62 + 0.38 * h.z);
                float wid = len * 0.15;
                float sway = sin(Time * 1.9 + e.z * 6.283 + root.x * 0.004) * 0.07 * Wind;
                float ang = -1.5708 + (e.x - 0.5) * 1.1 + g * (0.7 + 0.5 * e.y) + sway;
                vec2 d = vec2(cos(ang), sin(ang));
                vec2 n = vec2(-d.y, d.x);
                vec2 p = w - root;
                float t = dot(p, d) / len;
                if (t < 0.0 || t > 1.0) continue;
                float curve = ((e.y - 0.5) * 0.35 + g * 0.3) * len;
                float side = dot(p, n) - curve * t * t;
                float hw = wid * 0.5 * pow(1.0 - t, 0.65);
                float edge = abs(side) - hw;
                float rim = clamp(0.5 - (edge - max(aa * 0.7, wid * 0.18)) / aa, 0.0, 1.0);
                float body = clamp(0.5 - edge / aa, 0.0, 1.0);
                vec3 tone = mix(LOW, HIGH, smoothstep(0.0, 0.9, t)) * (0.78 + 0.44 * e.z) * (0.85 + 0.3 * patchy);
                tone = mix(tone, TIP, smoothstep(0.5, 1.0, t) * 0.55);
                tone *= 0.88 + 0.24 * clamp(0.5 + 0.5 * side / max(hw, 0.001), 0.0, 1.0);
                tone += SHEEN * g * t * 0.6;
                col = mix(col, col * 0.45, rim * fade * 0.7);
                col = mix(col, tone, body * fade);
            }
        }
    }
    return col;
}

void main() {
    float g = gust(world);
    vec3 col = turf(world, g);
    if (Fade.x > 0.0) col = blades(col, world, CELL_BIG, 3.1, Fade.x, g);
    if (Fade.y > 0.0) col = blades(col, world, CELL_MIDDLE, 7.7, Fade.y, g);
    if (Fade.z > 0.0) col = blades(col, world, CELL_SMALL, 1.3, Fade.z, g);
    gl_FragColor = vec4(col, 1.0);
}"#;

//the fragment shader with the sizes of the blades pasted in
pub(crate) fn grass_fragment_source() -> String {
    FRAGMENT.replace("CELL_BIG", &format!("{:?}", GRASS_CELLS[0])).replace("CELL_MIDDLE", &format!("{:?}", GRASS_CELLS[1])).replace("CELL_SMALL", &format!("{:?}", GRASS_CELLS[2]))
}

//how much of each size of blade is drawn when a world pixel is `pixels` pixels on the screen: a size whose blades are 0.4 to 0.9 pixels wide fades in,
//and one whose blades are 5 to 9 pixels wide fades out, but the smallest never does, so there is always grass close up
pub(crate) fn grass_fades(pixels: f32) -> [f32; 3] {
    let smooth = |from: f32, to: f32, x: f32| {
        let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let last = GRASS_CELLS.len() - 1;
    std::array::from_fn(|i| {
        let wide = GRASS_CELLS[i] * BLADE_SHARE * pixels;
        smooth(0.4, 0.9, wide) * if i == last { 1.0 } else { 1.0 - smooth(5.0, 9.0, wide) }
    })
}

#[repr(C)]
struct Uniforms {
    projection: [f32; 16],
    area: [f32; 4],
    pixels_per_unit: f32,
    time: f32,
    wind: f32,
    fade: [f32; 3],
}

struct Ground {
    pipeline: Pipeline,
    bindings: Bindings,
}

impl Ground {
    fn new(ctx: &mut dyn macroquad::miniquad::RenderingBackend) -> Option<Ground> {
        let fragment = grass_fragment_source();
        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: VERTEX, fragment: &fragment },
                ShaderMeta {
                    images: Vec::new(),
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("Projection", UniformType::Mat4),
                            UniformDesc::new("Area", UniformType::Float4),
                            UniformDesc::new("PixelsPerUnit", UniformType::Float1),
                            UniformDesc::new("Time", UniformType::Float1),
                            UniformDesc::new("Wind", UniformType::Float1),
                            UniformDesc::new("Fade", UniformType::Float3),
                        ],
                    },
                },
            )
            .inspect_err(|error| eprintln!("ground: the grass shader would not compile ({error:?}), the floor is flat"))
            .ok()?;
        let pipeline = ctx.new_pipeline(&[BufferLayout::default()], &[VertexAttribute::new("corner", VertexFormat::Float2)], shader, PipelineParams::default());
        let corners: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let corner_buffer = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(&corners));
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let index_buffer = ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&indices));
        Some(Ground { pipeline, bindings: Bindings { vertex_buffers: vec![corner_buffer], index_buffer, images: Vec::new() } })
    }
}

thread_local! {
    //None until the ground is first drawn, and None for good if this graphics card can't run the shader
    static GROUND: RefCell<Option<Option<Ground>>> = const { RefCell::new(None) };
}

//the grass over the part of the world in `view` (it stops at the world's edge), drawn with `camera`, which the world is drawn with. returns false if
//it can't be drawn, when the flat floor is drawn instead
pub(crate) fn draw_grass(view: Rect, camera: &Camera2D, time: f32) -> bool {
    let (left, top) = (view.x.max(0.0), view.y.max(0.0));
    let (right, bottom) = ((view.x + view.w).min(WORLD_SIZE), (view.y + view.h).min(WORLD_SIZE));
    if right <= left || bottom <= top {
        return true;
    }
    GROUND.with(|ground| {
        let mut ground = ground.borrow_mut();
        // SAFETY: macroquad's drawing is only done from this thread, and nothing it hands out is kept past this call
        let mut gl = unsafe { get_internal_gl() };
        let Some(ground) = ground.get_or_insert_with(|| Ground::new(gl.quad_context)).as_ref() else {
            return false;
        };
        //everything macroquad has been asked to draw so far is drawn first, so the grass goes over it
        gl.flush();
        let ctx = &mut *gl.quad_context;
        let (width, height) = macroquad::miniquad::window::screen_size();
        let pixels = camera.zoom.x.abs() * width / 2.0;
        let uniforms = Uniforms {
            projection: camera.matrix().to_cols_array(),
            area: [left, top, right, bottom],
            pixels_per_unit: pixels,
            time: time % 3600.0,
            wind: GRASS_WIND,
            fade: grass_fades(pixels),
        };
        ctx.begin_default_pass(PassAction::Nothing);
        ctx.apply_pipeline(&ground.pipeline);
        ctx.apply_viewport(0, 0, width as i32, height as i32);
        ctx.apply_scissor_rect(0, 0, width as i32, height as i32);
        ctx.apply_bindings(&ground.bindings);
        ctx.apply_uniforms(UniformsSource::table(&uniforms));
        ctx.draw(0, 6, 1);
        ctx.end_render_pass();
        true
    })
}
