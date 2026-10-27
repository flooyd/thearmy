//! the player as a little stick figure that runs while it moves and stands when it stops. it shows its front, its back or a side, depending on where it is going.
//! the pose is worked out from a stride phase each frame (`Runner::pose`, which needs no graphics card, so it is tested) and drawn with a few lines and circles
//! (`draw_runner`): at most four are on screen, so none of this needs the instanced batch. zoomed out it is drawn bigger, as the army is (see figure_scale),
//! and only out past that, too small to make out, is it the circle it was

use crate::prelude::*;

//the art, in pixels at zoom 1 (a unit's radius is 16), with the figure standing on the ground below its hip: x is to its right, y is down
pub(crate) const OUTLINE: f32 = 1.15; //the black edge round every part, outside it, as on the ships
const LIMB_WIDTH: f32 = 2.7;
const TORSO_WIDTH: f32 = 3.1;
const SHOULDER_WIDTH: f32 = 3.0;
pub(crate) const HEAD_RADIUS: f32 = 4.4;
const THIGH: f32 = 8.6;
const SHIN: f32 = 8.6;
const UPPER_ARM: f32 = 6.2;
const FOREARM: f32 = 6.0;
const TORSO: f32 = 11.5;
const GROUND: f32 = 17.6; //where its shadow is, below the hip
const FIGURE_LIFT: f32 = -2.0; //the figure is drawn this far above the player's position, so the middle of it is where the circle was
pub(crate) const EYE_RADIUS: f32 = 0.8;
pub(crate) const EDGE: Color = Color::new(0.024, 0.035, 0.05, 1.0);

//how much of the colour of the player each part of the figure is, so a limb on the far side is darker than one on the near side
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RunnerTone {
    Body,
    Dark,
    Light, //the head
    Back,  //the head seen from behind
}

//one thing to draw, in the order of drawing, as offsets in pixels from where the figure's hip is
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum RunnerPart {
    Shadow { half_width: f32 },
    Limb { points: [Vec2; 3], tone: RunnerTone, width: f32 },
    Head { at: Vec2, tone: RunnerTone, eyes: [Option<Vec2>; 2] },
}

//which way the figure faces the screen
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RunnerView {
    Front, //going down the screen, towards the viewer
    Back,  //going up the screen
    Side,  //going left or right, which is `side`
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Runner {
    pub(crate) phase: f32, //where it is in a stride, 0 to a full turn: one foot down, then the other
    pub(crate) run: f32,   //0 standing, 1 running, eased between
    pub(crate) view: RunnerView,
    pub(crate) side: f32, //1 for facing right and -1 for left, which a side view keeps while it is going up or down
    pub(crate) flip: f32, //where it is turning from one side to the other, eased
    pub(crate) lean: f32, //how far forward it leans, more when it is going faster
    pub(crate) time: f32, //for the breathing while it stands
}

impl Runner {
    pub(crate) fn new() -> Runner {
        Runner { phase: 0.0, run: 0.0, view: RunnerView::Front, side: 1.0, flip: 1.0, lean: RUNNER_LEAN, time: 0.0 }
    }

    //a frame later, for a player moving at `velocity`. how fast they are going says how quickly it strides: no faster than the speed with no army boost
    //is the steady pace, and each boost on top of that quickens it a little (not by as much as the ground goes by, which would be a blur)
    pub(crate) fn step(&mut self, velocity: Vec2, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let speed = velocity.length();
        let boost = (speed / PLAYER_SPEED).clamp(1.0, 3.0);
        let target = if speed > RUNNER_MIN_SPEED { 1.0 } else { 0.0 };
        self.run += (target - self.run) * (1.0 - (-RUNNER_EASE * dt).exp());
        let strides = (RUNNER_CADENCE + RUNNER_BOOST_CADENCE * (boost - 1.0)) * dt * self.run;
        self.phase = (self.phase + strides * std::f32::consts::TAU).rem_euclid(std::f32::consts::TAU);
        self.time = (self.time + dt) % 1000.0;
        self.lean = RUNNER_LEAN + RUNNER_LEAN_BOOST * (boost - 1.0);
        if speed > RUNNER_TURN_SPEED {
            self.turn_to(velocity / speed);
        }
        self.flip += (self.side - self.flip) * (1.0 - (-RUNNER_FLIP * dt).exp());
    }

    //face the way it is going. a side view changes to a front or back one only when the way it is going is clearly more up or down than across, and the
    //other way round, so a player going near the diagonal does not flicker between the two
    fn turn_to(&mut self, heading: Vec2) {
        if heading.x > 0.15 {
            self.side = 1.0;
        } else if heading.x < -0.15 {
            self.side = -1.0;
        }
        let (across, along) = (heading.x.abs(), heading.y.abs());
        self.view = match self.view {
            RunnerView::Side if along > across * RUNNER_VIEW_HOLD => if heading.y > 0.0 { RunnerView::Front } else { RunnerView::Back },
            RunnerView::Side => RunnerView::Side,
            _ if across > along * RUNNER_VIEW_HOLD => RunnerView::Side,
            _ => if heading.y > 0.0 { RunnerView::Front } else { RunnerView::Back },
        };
    }

    //the parts of the figure, from the back to the front
    pub(crate) fn pose(&self) -> Vec<RunnerPart> {
        match self.view {
            RunnerView::Side => self.side_pose(),
            RunnerView::Front => self.front_pose(false),
            RunnerView::Back => self.front_pose(true),
        }
    }

    fn bob(&self, lift: f32) -> f32 {
        //up and down twice in a stride, highest when the legs are most apart, and a slow breath while it stands
        self.run * lift * (1.0 - (2.0 * self.phase).cos()) / 2.0 + (1.0 - self.run) * 0.45 * (self.time * 2.2).sin()
    }

    //in profile: angles are from straight down, forward positive, and the whole figure is then turned to the side it faces
    fn side_pose(&self) -> Vec<RunnerPart> {
        let (p, run, t) = (self.phase, self.run, self.time);
        let pi = std::f32::consts::PI;
        let bob = self.bob(1.5);
        let hip = Vec2::new(0.0, -bob);
        let lean = self.lean * run + 0.05;
        let up = Vec2::new(lean.sin(), -lean.cos());
        let neck = hip + up * TORSO;
        let shoulder = hip + up * (TORSO * 0.9);
        let at = |angle: f32| Vec2::new(angle.sin(), angle.cos());
        let leg = |q: f32, s: f32| {
            let thigh = run * (0.95 * q.sin() + 0.1) + (1.0 - run) * s * 0.1;
            //the knee bends most as the foot swings forward under the body, and hardly at all while it is on the ground
            let bend = run * (0.2 + 1.3 * (q - 0.25).cos().max(0.0));
            let knee = hip + at(thigh) * THIGH;
            [hip, knee, knee + at(thigh - bend) * SHIN]
        };
        let arm = |q: f32, s: f32| {
            let swing = -0.95 * run * q.sin() + (1.0 - run) * 0.06 * (t * 1.7 + s).sin();
            let bend = lerp(0.28, 1.4 + 0.25 * q.sin(), run);
            let elbow = shoulder + at(swing) * UPPER_ARM;
            [shoulder, elbow, elbow + at(swing + bend) * FOREARM]
        };
        let head = neck + up * (HEAD_RADIUS + 1.2);
        let parts = vec![
            RunnerPart::Shadow { half_width: shadow_width(bob, run) },
            RunnerPart::Limb { points: arm(p, -1.0), tone: RunnerTone::Dark, width: LIMB_WIDTH },
            RunnerPart::Limb { points: leg(p + pi, -1.0), tone: RunnerTone::Dark, width: LIMB_WIDTH },
            RunnerPart::Limb { points: [hip, neck, neck], tone: RunnerTone::Body, width: TORSO_WIDTH },
            RunnerPart::Head { at: head, tone: RunnerTone::Light, eyes: [Some(head + Vec2::new(2.0, -0.5)), None] },
            RunnerPart::Limb { points: leg(p, 1.0), tone: RunnerTone::Body, width: LIMB_WIDTH },
            RunnerPart::Limb { points: arm(p + pi, 1.0), tone: RunnerTone::Body, width: LIMB_WIDTH },
        ];
        //(a figure turning round passes through being edge on, which is thin but never nothing)
        let across = if self.flip.abs() < 0.04 { 0.04 * self.flip.signum() } else { self.flip };
        parts.into_iter().map(|part| squeezed(part, across)).collect()
    }

    //facing the viewer, or with its back to them: the legs take turns to lift and the arms pump
    fn front_pose(&self, back: bool) -> Vec<RunnerPart> {
        let (p, run) = (self.phase, self.run);
        let pi = std::f32::consts::PI;
        let bob = self.bob(1.3);
        let hip = Vec2::new(0.0, -bob);
        let sway = run * 0.9 * p.sin();
        let shoulder = Vec2::new(sway * 0.5, hip.y - TORSO * 0.89);
        let neck = Vec2::new(sway * 0.6, hip.y - TORSO - 0.1);
        let mut legs: Vec<(f32, [Vec2; 3])> = [1.0f32, -1.0]
            .into_iter()
            .map(|s| {
                let q = p + if s > 0.0 { 0.0 } else { pi };
                let lift = run * q.sin().max(0.0) * 6.2;
                let knee = Vec2::new(s * (3.3 + lift * 0.14), hip.y + 8.3 - lift * 0.55);
                let foot = Vec2::new(s * (3.7 - lift * 0.05), hip.y + 16.6 - lift * 1.2);
                (lift, [Vec2::new(s * 2.0, hip.y), knee, foot])
            })
            .collect();
        //(the leg that is lifted is the one nearer, so it is drawn last)
        legs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut parts = vec![RunnerPart::Shadow { half_width: shadow_width(bob, run) }];
        for (lift, points) in legs {
            parts.push(RunnerPart::Limb { points, tone: if lift > 1.5 { RunnerTone::Body } else { RunnerTone::Dark }, width: LIMB_WIDTH });
        }
        parts.push(RunnerPart::Limb { points: [hip, neck, neck], tone: RunnerTone::Body, width: TORSO_WIDTH });
        parts.push(RunnerPart::Limb { points: [shoulder - Vec2::new(3.4, 0.0), shoulder + Vec2::new(3.4, 0.0), shoulder + Vec2::new(3.4, 0.0)], tone: RunnerTone::Body, width: SHOULDER_WIDTH });
        for s in [1.0f32, -1.0] {
            let q = p + if s > 0.0 { 0.0 } else { pi } + pi;
            let elbow = shoulder + Vec2::new(s * 6.6, 5.2);
            let hand = shoulder + Vec2::new(s * lerp(6.2, 4.8 + 0.8 * q.cos(), run), lerp(10.4, 5.8 - 5.2 * q.sin(), run));
            parts.push(RunnerPart::Limb { points: [shoulder + Vec2::new(s * 3.4, 0.0), elbow, hand], tone: RunnerTone::Body, width: LIMB_WIDTH });
        }
        let head = neck - Vec2::new(0.0, HEAD_RADIUS + 1.0);
        let eyes = if back { [None, None] } else { [Some(head + Vec2::new(1.7, -0.3)), Some(head + Vec2::new(-1.7, -0.3))] };
        parts.push(RunnerPart::Head { at: head, tone: if back { RunnerTone::Back } else { RunnerTone::Light }, eyes });
        parts
    }
}

fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}

fn shadow_width(bob: f32, run: f32) -> f32 {
    8.4 - bob * 0.5 - run * 0.4
}

//a part with everything across it (x) scaled, which turns it to face the other way when that is negative
fn squeezed(part: RunnerPart, across: f32) -> RunnerPart {
    let squeeze = |point: Vec2| Vec2::new(point.x * across, point.y);
    match part {
        RunnerPart::Shadow { .. } => part,
        RunnerPart::Limb { points, tone, width } => RunnerPart::Limb { points: points.map(squeeze), tone, width },
        RunnerPart::Head { at, tone, eyes } => RunnerPart::Head { at: squeeze(at), tone, eyes: eyes.map(|eye| eye.map(squeeze)) },
    }
}

//whether a unit radius this many pixels on the screen is big enough to make a figure out. under that a player is the circle they were, and never both: a
//circle left round a figure reads as a ring round the player
pub(crate) fn runner_shown(screen_radius: f32) -> bool {
    screen_radius >= RUNNER_MIN_RADIUS
}

pub(crate) fn tone_color(tone: RunnerTone, color: Color) -> Color {
    let shaded = match tone {
        RunnerTone::Body => color,
        RunnerTone::Dark => lerp_color(color, EDGE, 0.3),
        RunnerTone::Light => lerp_color(color, WHITE, 0.3),
        RunnerTone::Back => lerp_color(color, EDGE, 0.12),
    };
    Color::new(shaded.r, shaded.g, shaded.b, 1.0)
}

//a line of up to three points with round joints and ends
fn draw_stroke(at: Vec2, points: &[Vec2; 3], width: f32, color: Color) {
    for pair in points.windows(2) {
        if pair[0] != pair[1] {
            draw_line(at.x + pair[0].x, at.y + pair[0].y, at.x + pair[1].x, at.y + pair[1].y, width, color);
        }
    }
    for (i, point) in points.iter().enumerate() {
        if i == 0 || *point != points[i - 1] {
            draw_circle(at.x + point.x, at.y + point.y, width / 2.0, color);
        }
    }
}

//the figure at a player's position, in the world (the camera does the zooming), `scale` times its size (see figure_scale)
pub(crate) fn draw_runner(runner: &Runner, position: Vec2, color: Color, scale: f32) {
    let hip = position + Vec2::new(0.0, FIGURE_LIFT * scale);
    let edge = EDGE;
    for part in runner.pose() {
        match part {
            RunnerPart::Shadow { half_width } => draw_ellipse(hip.x, hip.y + GROUND * scale, half_width * scale, 2.5 * scale, 0.0, Color::new(0.0, 0.0, 0.0, 0.38)),
            RunnerPart::Limb { points, tone, width } => {
                let points = points.map(|point| point * scale);
                draw_stroke(hip, &points, (width + 2.0 * OUTLINE) * scale, edge);
                draw_stroke(hip, &points, width * scale, tone_color(tone, color));
            }
            RunnerPart::Head { at, tone, eyes } => {
                let at = hip + at * scale;
                draw_circle(at.x, at.y, (HEAD_RADIUS + OUTLINE) * scale, edge);
                draw_circle(at.x, at.y, HEAD_RADIUS * scale, tone_color(tone, color));
                for eye in eyes.into_iter().flatten() {
                    draw_circle(hip.x + eye.x * scale, hip.y + eye.y * scale, EYE_RADIUS * scale, edge);
                }
            }
        }
    }
}
