//! the ground the world stands on: a meadow of grass, drawn by the graphics card in one go over the part of the world in view. the blades come in three
//! sizes and each zoom shows the ones that come out about a pixel or more wide, fading from one size to the next, so it is grass all the way out. gusts of
//! wind roll across it, bending the blades and lighting them, and big sunny and shaded patches and bare spots keep it from looking tiled. it is the look
//! the owner chose from the mock-up ("Deep meadow"): dark, so the army, the warnings and the babies stand out on it. the patches of ice, lava and swamp
//! (see biomes.rs) are drawn over it in the same go: cracked ice with drifts of snow and glints, dark crust with glowing seams and pools of molten rock,
//! and murky water with weed and ripples, each with an edge the grass round it shows (frost, scorch and mud). a card that can't run the shader keeps the
//! flat floor (see `draw_world`)

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
uniform vec4 Patches[MAX_PATCHES];
uniform float PatchCount;

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

// ---------- the patches of ice, lava and swamp ----------

//what moves the edge of a patch, as `edge_noise` in biomes.rs
float edgeNoise(vec2 w) {
    vec2 p = w * EDGE_SCALE;
    return 0.65 * noise(p) + 0.35 * noise(p * 2.03 + vec2(17.1, 9.3));
}

//how far into a patch a place is, as a share of its radius (under 1 is on it), as `Patch::depth` in biomes.rs
float depthIn(vec4 spot, vec2 w) {
    float d = length(w - spot.xy) / spot.z;
    if (abs(d - 1.0) > WOBBLE) return d;
    return d + (edgeNoise(w) - 0.5) * 2.0 * WOBBLE;
}

//how far a place is from the nearest seam between the cells of a crazy paving, in cells (0 on a seam)
float seam(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    float d1 = 8.0;
    float d2 = 8.0;
    for (int y = -1; y < 2; y++) {
        for (int x = -1; x < 2; x++) {
            vec2 o = vec2(float(x), float(y));
            vec2 r = o + 0.1 + 0.8 * hash32(i + o).xy - f;
            float d = dot(r, r);
            if (d < d1) {
                d2 = d1;
                d1 = d;
            } else if (d < d2) {
                d2 = d;
            }
        }
    }
    return (sqrt(d2) - sqrt(d1)) * 0.5;
}

//ice: blue-grey, lighter in patches, with drifts of snow, cracks big and small (the small ones only once they are big enough to see), a glint here and
//there, and a pale rim
vec3 ice(vec2 w, float depth, float px) {
    vec3 c = mix(vec3(0.30, 0.43, 0.53), vec3(0.47, 0.61, 0.71), noise(w * 0.005));
    c = mix(c, vec3(0.74, 0.82, 0.88), smoothstep(0.56, 0.76, fbm(w * 0.0022 + 5.0)) * 0.85);
    float big = seam(w / 230.0) * 230.0;
    c *= 1.0 - 0.38 * (1.0 - smoothstep(0.0, px * 1.6, big));
    c += vec3(0.10, 0.12, 0.13) * (1.0 - smoothstep(px * 1.6, px * 3.5, big));
    float small = seam(w / 75.0 + 3.7) * 75.0;
    c *= 1.0 - 0.22 * (1.0 - smoothstep(0.0, px * 1.2, small)) * smoothstep(8.0, 20.0, 75.0 * PixelsPerUnit);
    vec2 cell = floor(w / 30.0);
    vec3 h = hash32(cell + 7.7);
    float glint = pow(max(0.0, sin(Time * 2.3 + h.z * 40.0)), 24.0) * step(0.86, h.x);
    c += vec3(0.9, 0.95, 1.0) * glint * (1.0 - smoothstep(0.0, max(px * 1.8, 3.0), length(w - (cell + 0.2 + 0.6 * h.yz) * 30.0)));
    return mix(c, vec3(0.80, 0.88, 0.94), smoothstep(0.90, 1.0, depth) * 0.55);
}

//lava: plates of dark crust with seams that glow and throb, and pools of molten rock that flow; the crust nearest the edge has cooled darker
vec3 lava(vec2 w, float depth, float px) {
    float flow = fbm(w * 0.0019 + vec2(Time * 0.021, Time * 0.014));
    float pulse = 0.72 + 0.28 * sin(Time * 1.6 + flow * 11.0);
    vec3 crust = mix(vec3(0.07, 0.05, 0.045), vec3(0.15, 0.10, 0.08), noise(w * 0.02)) * (0.8 + 0.4 * noise(w * 0.006));
    float plates = seam(w / 170.0) * 170.0;
    vec3 glow = vec3(1.0, 0.34, 0.05) * pulse;
    float line = min(px * 1.6, 9.0);
    vec3 c = crust + glow * (1.0 - smoothstep(line * 0.5, line, plates)) * 0.85 + glow * (1.0 - smoothstep(0.0, 22.0, plates)) * 0.12;
    float pool = smoothstep(0.66, 0.74, flow);
    vec3 molten = mix(vec3(0.85, 0.24, 0.03), vec3(1.0, 0.72, 0.22), smoothstep(0.66, 0.86, flow + 0.04 * sin(Time * 2.1 + w.x * 0.01)));
    c = mix(c, molten * (0.85 + 0.15 * pulse), pool);
    return mix(c, crust * 0.65, smoothstep(0.86, 1.0, depth) * (1.0 - pool));
}

//swamp: murky water, darker and lighter, mats of weed, rings spreading where something stirs, the gusts shivering on it, and a mud bank at the edge
vec3 swamp(vec2 w, float depth, float px, float g) {
    vec3 c = mix(vec3(0.055, 0.09, 0.07), vec3(0.11, 0.15, 0.09), fbm(w * 0.003 + 2.0));
    float weed = smoothstep(0.55, 0.74, fbm(w * 0.0042 + 9.0));
    float fleck = step(0.55, hash12(floor(w / 10.0)));
    c = mix(c, vec3(0.26, 0.40, 0.12), weed * (0.6 + 0.4 * mix(1.0, fleck, smoothstep(4.0, 9.0, 10.0 * PixelsPerUnit))));
    vec2 cell = floor(w / 260.0);
    vec3 h = hash32(cell + 3.3);
    float phase = fract(Time * 0.22 + h.z);
    float ring = abs(length(w - (cell + 0.25 + 0.5 * h.xy) * 260.0) - phase * 110.0);
    c += vec3(0.10, 0.13, 0.10) * (1.0 - smoothstep(0.0, max(px * 1.5, 2.0), ring)) * (1.0 - phase) * (1.0 - weed);
    c += vec3(0.05, 0.06, 0.04) * g * (1.0 - weed);
    return mix(c, vec3(0.15, 0.11, 0.065), smoothstep(0.84, 0.99, depth));
}

void main() {
    float g = gust(world);
    float px = 1.0 / PixelsPerUnit;
    //the patch this is on or near, if any (they don't touch, so there is one at most), and how far into it
    float depth = 9.0;
    float kind = 0.0;
    float radius = 1.0;
    for (int k = 0; k < MAX_PATCHES; k++) {
        if (float(k) >= PatchCount) break;
        vec4 spot = Patches[k];
        if (length(world - spot.xy) < spot.z * (1.0 + WOBBLE + RIM)) {
            depth = depthIn(spot, world);
            kind = spot.w;
            radius = spot.z;
        }
    }
    float edge = max(px * 1.2 / radius, 0.002) * (kind > 2.5 ? 3.0 : 1.0);
    float on = 1.0 - smoothstep(1.0 - edge, 1.0 + edge, depth);
    vec3 col = vec3(0.0);
    if (on < 1.0) {
        col = turf(world, g);
        if (Fade.x > 0.0) col = blades(col, world, CELL_BIG, 3.1, Fade.x, g);
        if (Fade.y > 0.0) col = blades(col, world, CELL_MIDDLE, 7.7, Fade.y, g);
        if (Fade.z > 0.0) col = blades(col, world, CELL_SMALL, 1.3, Fade.z, g);
        //the grass round a patch: frosted round ice, scorched round lava, sodden round a swamp
        float rim = (1.0 - smoothstep(1.0, 1.0 + RIM, depth)) * step(0.5, kind);
        float speckle = 0.6 + 0.4 * noise(world * 0.03);
        if (kind > 0.5 && kind < 1.5) col = mix(col, vec3(0.72, 0.80, 0.86), rim * 0.6 * speckle);
        if (kind > 1.5 && kind < 2.5) col = mix(col, vec3(0.05, 0.04, 0.035), rim * 0.8 * speckle);
        if (kind > 2.5) col = mix(col, col * 0.55 + vec3(0.05, 0.035, 0.0), rim * 0.8);
    }
    if (on > 0.0) {
        vec3 ground = kind < 1.5 ? ice(world, depth, px) : (kind < 2.5 ? lava(world, depth, px) : swamp(world, depth, px, g));
        col = mix(col, ground, on);
    }
    gl_FragColor = vec4(col, 1.0);
}"#;

//how far past the edge of a patch the grass round it shows it, as a share of its radius
const RIM: f32 = 0.07;

//the fragment shader with the sizes of the blades and the patches' numbers pasted in
pub(crate) fn grass_fragment_source() -> String {
    FRAGMENT
        .replace("CELL_BIG", &format!("{:?}", GRASS_CELLS[0]))
        .replace("CELL_MIDDLE", &format!("{:?}", GRASS_CELLS[1]))
        .replace("CELL_SMALL", &format!("{:?}", GRASS_CELLS[2]))
        .replace("MAX_PATCHES", &MAX_PATCHES_DRAWN.to_string())
        .replace("EDGE_SCALE", &format!("{:?}", PATCH_EDGE_SCALE))
        .replace("WOBBLE", &format!("{:?}", PATCH_WOBBLE))
        .replace("RIM", &format!("{RIM:?}"))
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
    patches: [[f32; 4]; MAX_PATCHES_DRAWN],
    patch_count: f32,
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
                            UniformDesc::new("Patches", UniformType::Float4).array(MAX_PATCHES_DRAWN),
                            UniformDesc::new("PatchCount", UniformType::Float1),
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

//the grass over the part of the world in `view` (it stops at the world's edge), with the `patches` there (no more than MAX_PATCHES_DRAWN), drawn with
//`camera`, which the world is drawn with. returns false if it can't be drawn, when the flat floor is drawn instead
pub(crate) fn draw_grass(view: Rect, camera: &Camera2D, time: f32, patches: &[Patch]) -> bool {
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
            patches: std::array::from_fn(|k| patches.get(k).map_or([0.0; 4], |patch| [patch.position.x, patch.position.y, patch.radius, patch.biome.code()])),
            patch_count: patches.len().min(MAX_PATCHES_DRAWN) as f32,
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
