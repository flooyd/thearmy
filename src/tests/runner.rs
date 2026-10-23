//! the stick figure that is the player: which way it faces, how it strides, and that every pose it can take is a sound one. what is drawn is not tested (there is
//! no graphics card), but everything that decides it is

use super::*;

const DT: f32 = 1.0 / 120.0;

fn running(runner: &mut Runner, velocity: Vec2, seconds: f32) {
    for _ in 0..(seconds / DT) as usize {
        runner.step(velocity, DT);
    }
}

//every point of a pose
fn points(pose: &[RunnerPart]) -> Vec<Vec2> {
    pose.iter()
        .flat_map(|part| match *part {
            RunnerPart::Shadow { .. } => Vec::new(),
            RunnerPart::Limb { points, .. } => points.to_vec(),
            RunnerPart::Head { at, eyes, .. } => std::iter::once(at).chain(eyes.into_iter().flatten()).collect(),
        })
        .collect()
}

//a runner in the middle of a stride, whatever is asked
fn posed(view: RunnerView, phase: f32, run: f32, flip: f32) -> Runner {
    Runner { phase, run, view, side: flip.signum(), flip, lean: RUNNER_LEAN, time: 0.7 }
}

#[test]
fn a_new_figure_stands_facing_the_viewer() {
    let runner = Runner::new();
    assert_eq!((runner.run, runner.phase, runner.view), (0.0, 0.0, RunnerView::Front));
    assert!(Feedback::new().runners.is_empty());
}

#[test]
fn a_player_standing_still_does_not_stride_and_one_going_somewhere_does() {
    let mut runner = Runner::new();
    running(&mut runner, Vec2::ZERO, 1.0);
    assert_eq!((runner.phase, runner.run), (0.0, 0.0));
    running(&mut runner, Vec2::new(PLAYER_SPEED, 0.0), 0.5);
    assert!(runner.run > 0.95, "{}", runner.run);
    assert!(runner.phase > 0.0 && runner.phase < std::f32::consts::TAU);
}

#[test]
fn it_stops_where_it_is_when_the_player_stops_and_comes_to_rest() {
    let mut runner = Runner::new();
    running(&mut runner, Vec2::new(0.0, PLAYER_SPEED), 1.0);
    running(&mut runner, Vec2::ZERO, 1.5);
    assert!(runner.run < 0.001, "{}", runner.run);
    let held = runner.phase;
    running(&mut runner, Vec2::ZERO, 1.0);
    assert!((runner.phase - held).abs() < 0.001, "it does not carry on striding in place");
    assert_eq!(runner.view, RunnerView::Front, "and it keeps facing the way it was going");
}

#[test]
fn a_boost_quickens_the_stride_but_far_less_than_it_quickens_the_ground() {
    let strides = |speed: f32| {
        let mut runner = Runner { run: 1.0, ..Runner::new() };
        let mut total = 0.0;
        let mut last = runner.phase;
        for _ in 0..240 {
            runner.step(Vec2::new(speed, 0.0), DT);
            total += (runner.phase - last).rem_euclid(std::f32::consts::TAU);
            last = runner.phase;
        }
        total / std::f32::consts::TAU / 2.0
    };
    let (steady, boosted, most) = (strides(PLAYER_SPEED), strides(PLAYER_SPEED * 2.0), strides(PLAYER_SPEED * 3.0));
    assert!((steady - RUNNER_CADENCE).abs() < 0.05, "{steady}");
    assert!((boosted - (RUNNER_CADENCE + RUNNER_BOOST_CADENCE)).abs() < 0.05, "{boosted}");
    assert!(steady < boosted && boosted < most);
    assert!(most / steady < 2.0, "three times the speed is not three times the strides: {steady} to {most}");
    //a player going slower than the steady speed (easing to a stop, or slowed) strides at the steady pace, not slower
    assert!((strides(PLAYER_SPEED * 0.3) - RUNNER_CADENCE).abs() < 0.05);
}

#[test]
fn the_phase_stays_inside_one_stride() {
    let mut runner = Runner::new();
    for i in 0..5000 {
        let angle = i as f32 * 0.01;
        runner.step(Vec2::from_angle(angle) * PLAYER_SPEED * 3.0, DT);
        assert!((0.0..std::f32::consts::TAU).contains(&runner.phase), "{}", runner.phase);
        assert!(runner.time < 1000.0 && runner.flip.abs() <= 1.0 + 1e-4);
    }
}

#[test]
fn it_faces_front_going_down_back_going_up_and_a_side_going_across() {
    let mut runner = Runner::new();
    let going = |runner: &mut Runner, x: f32, y: f32| {
        running(runner, Vec2::new(x, y), 0.3);
        (runner.view, runner.side)
    };
    assert_eq!(going(&mut runner, 400.0, 0.0), (RunnerView::Side, 1.0));
    assert_eq!(going(&mut runner, 0.0, 400.0).0, RunnerView::Front);
    assert_eq!(going(&mut runner, -400.0, 0.0), (RunnerView::Side, -1.0));
    assert_eq!(going(&mut runner, 0.0, -400.0).0, RunnerView::Back);
    assert!(runner.flip < 0.0 || runner.side < 0.0, "it keeps the side it last went to");
}

#[test]
fn it_keeps_its_side_going_up_and_down_and_turns_when_it_changes_side() {
    let mut runner = Runner::new();
    running(&mut runner, Vec2::new(-400.0, 0.0), 0.5);
    assert!(runner.flip < -0.99, "{}", runner.flip);
    running(&mut runner, Vec2::new(0.0, 400.0), 0.5);
    running(&mut runner, Vec2::new(0.0, 0.0), 0.5);
    //(standing with its face to the viewer, the side it will take when it next goes across is still the one it had)
    assert_eq!(runner.side, -1.0);
    running(&mut runner, Vec2::new(400.0, 0.0), DT * 6.0);
    assert!(runner.flip > -1.0 && runner.flip < 1.0, "turning, not jumping: {}", runner.flip);
    running(&mut runner, Vec2::new(400.0, 0.0), 0.5);
    assert!(runner.flip > 0.99);
}

#[test]
fn a_player_going_along_a_diagonal_does_not_flicker_between_views() {
    //at 45 degrees neither way is clearly more than the other, so it keeps whichever it had
    let diagonal = Vec2::new(300.0, 300.0);
    let mut side = Runner::new();
    running(&mut side, Vec2::new(400.0, 0.0), 0.3);
    running(&mut side, diagonal, 1.0);
    assert_eq!(side.view, RunnerView::Side);
    let mut front = Runner::new();
    running(&mut front, Vec2::new(0.0, 400.0), 0.3);
    running(&mut front, diagonal, 1.0);
    assert_eq!(front.view, RunnerView::Front);
    //and a wobble in the heading on the diagonal does not change it either
    let mut wobbling = Runner::new();
    running(&mut wobbling, Vec2::new(400.0, 0.0), 0.3);
    for i in 0..600 {
        let angle = std::f32::consts::FRAC_PI_4 + 0.1 * (i as f32 * 0.7).sin();
        wobbling.step(Vec2::from_angle(angle) * 400.0, DT);
        assert_eq!(wobbling.view, RunnerView::Side, "frame {i}");
    }
    //but clearly more up than across is a change
    running(&mut side, Vec2::new(100.0, 140.0), 0.2);
    assert_eq!(side.view, RunnerView::Front);
}

#[test]
fn a_player_easing_to_a_stop_does_not_turn_the_figure() {
    let mut runner = Runner::new();
    running(&mut runner, Vec2::new(400.0, 0.0), 0.5);
    //the last of the movement is a crawl in a direction that means nothing
    running(&mut runner, Vec2::new(-5.0, 15.0), 1.0);
    assert_eq!((runner.view, runner.side), (RunnerView::Side, 1.0));
}

#[test]
fn no_time_is_no_change() {
    let mut runner = Runner::new();
    running(&mut runner, Vec2::new(0.0, -300.0), 0.4);
    let before = runner.clone();
    runner.step(Vec2::new(300.0, 0.0), 0.0);
    runner.step(Vec2::new(300.0, 0.0), -1.0);
    assert_eq!(runner, before, "a paused game does not stride");
}

#[test]
fn every_pose_it_can_take_is_a_sound_one() {
    for view in [RunnerView::Front, RunnerView::Back, RunnerView::Side] {
        for run in [0.0, 0.5, 1.0] {
            for flip in [1.0, -1.0, 0.0, 0.3] {
                for step in 0..64 {
                    let phase = step as f32 * std::f32::consts::TAU / 64.0;
                    let pose = posed(view, phase, run, flip).pose();
                    for point in points(&pose) {
                        assert!(point.is_finite(), "{view:?} {phase} {run}");
                        //it fits in the box it is drawn in, whose top is what the names above it make room for
                        assert!(point.x.abs() < 15.0 && point.y > -(RUNNER_TOP - 3.0) && point.y < 19.0, "{view:?} phase {phase} run {run} flip {flip}: {point}");
                    }
                    for part in &pose {
                        if let RunnerPart::Limb { width, .. } = part {
                            assert!(*width > 0.0);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn a_limb_is_as_long_in_every_pose_as_it_is_in_any_other() {
    //a bone is the same length however the figure moves, or it would look stretched. (the front view foreshortens, so only the side view is held to it)
    for step in 0..64 {
        let phase = step as f32 * std::f32::consts::TAU / 64.0;
        let pose = posed(RunnerView::Side, phase, 1.0, 1.0).pose();
        let limbs: Vec<[Vec2; 3]> = pose.iter().filter_map(|part| if let RunnerPart::Limb { points, width, .. } = part { (*width == 2.7).then_some(*points) } else { None }).collect();
        assert_eq!(limbs.len(), 4, "two arms and two legs");
        for (i, limb) in limbs.iter().enumerate() {
            let (upper, lower) = (limb[0].distance(limb[1]), limb[1].distance(limb[2]));
            //(the far arm, the far leg, the near leg and the near arm, in that order)
            let (want_upper, want_lower) = if i == 1 || i == 2 { (8.6, 8.6) } else { (6.2, 6.0) };
            assert!((upper - want_upper).abs() < 1e-3 && (lower - want_lower).abs() < 1e-3, "limb {i} at {phase}: {upper} and {lower}");
        }
    }
}

#[test]
fn its_feet_stay_near_the_ground_and_its_head_above_everything() {
    for view in [RunnerView::Front, RunnerView::Back, RunnerView::Side] {
        for step in 0..64 {
            let phase = step as f32 * std::f32::consts::TAU / 64.0;
            let pose = posed(view, phase, 1.0, 1.0).pose();
            let head = pose.iter().find_map(|part| if let RunnerPart::Head { at, .. } = part { Some(*at) } else { None }).unwrap();
            let limbs = pose.iter().filter_map(|part| if let RunnerPart::Limb { points, .. } = part { Some(*points) } else { None });
            let highest = limbs.flatten().map(|point| point.y).fold(f32::MAX, f32::min);
            assert!(head.y < highest, "{view:?} at {phase}: the head is the top of it");
        }
    }
    //standing, the feet are on the ground the shadow is on
    let standing = posed(RunnerView::Front, 0.0, 0.0, 1.0).pose();
    let feet: Vec<f32> = standing.iter().filter_map(|part| if let RunnerPart::Limb { points, width, .. } = part { (*width == 2.7 && points[2].y > 10.0).then_some(points[2].y) } else { None }).collect();
    assert_eq!(feet.len(), 2);
    assert!(feet.iter().all(|y| (y - 16.6).abs() < 1.0), "{feet:?}");
}

#[test]
fn what_is_drawn_is_in_order_with_the_shadow_first_and_the_right_eyes() {
    let count = |view: RunnerView| {
        let pose = posed(view, 1.0, 1.0, 1.0).pose();
        let eyes = pose.iter().map(|part| if let RunnerPart::Head { eyes, .. } = part { eyes.iter().flatten().count() } else { 0 }).sum::<usize>();
        (pose.len(), matches!(pose[0], RunnerPart::Shadow { .. }), eyes)
    };
    assert_eq!(count(RunnerView::Side), (7, true, 1), "one eye in profile");
    assert_eq!(count(RunnerView::Front), (8, true, 2));
    assert_eq!(count(RunnerView::Back), (8, true, 0), "none from behind");
    let back = posed(RunnerView::Back, 1.0, 1.0, 1.0).pose();
    assert!(matches!(back.last(), Some(RunnerPart::Head { tone: RunnerTone::Back, .. })));
}

#[test]
fn facing_left_is_facing_right_the_other_way_round() {
    for step in 0..16 {
        let phase = step as f32 * std::f32::consts::TAU / 16.0;
        let right = posed(RunnerView::Side, phase, 1.0, 1.0).pose();
        let left = posed(RunnerView::Side, phase, 1.0, -1.0).pose();
        for (a, b) in points(&right).into_iter().zip(points(&left)) {
            assert!((a.x + b.x).abs() < 1e-4 && (a.y - b.y).abs() < 1e-4, "{a} against {b}");
        }
    }
}

#[test]
fn the_figure_lifts_a_foot_at_a_time_and_swings_the_arm_opposite_it() {
    //front view: in one half of the stride one foot is higher than the other, and in the other half it is the other way round
    let height_of_feet = |phase: f32| {
        let pose = posed(RunnerView::Front, phase, 1.0, 1.0).pose();
        let mut feet: Vec<Vec2> = pose.iter().filter_map(|part| if let RunnerPart::Limb { points, width, .. } = part { (*width == 2.7 && points[2].y > 5.0).then_some(points[2]) } else { None }).collect();
        feet.sort_by(|a, b| a.x.total_cmp(&b.x));
        (feet[0].y, feet[1].y)
    };
    let (left, right) = height_of_feet(std::f32::consts::FRAC_PI_2);
    assert!((left - right).abs() > 3.0, "one is lifted: {left} and {right}");
    let (left2, right2) = height_of_feet(std::f32::consts::FRAC_PI_2 * 3.0);
    assert!((left2 - right2).abs() > 3.0 && (left - right).signum() != (left2 - right2).signum(), "and then the other");
}

#[test]
fn the_figure_is_a_circle_when_small_a_figure_when_big_and_a_fade_between() {
    assert_eq!(runner_show(0.0), 0.0);
    assert_eq!(runner_show(RUNNER_FADE_FROM), 0.0);
    assert_eq!(runner_show(RUNNER_FADE_FROM + RUNNER_FADE_RANGE), 1.0);
    assert_eq!(runner_show(RADIUS * 2.5), 1.0, "zoomed all the way in it is the figure");
    assert_eq!(runner_show(RADIUS * MIN_ZOOM), 0.0, "and all the way out the circle");
    let mut last = 0.0;
    for step in 0..=100 {
        let show = runner_show(step as f32 * 0.15);
        assert!(show >= last && (0.0..=1.0).contains(&show));
        last = show;
    }
    //at the zoom the game starts at (a unit is 16 pixels across the radius) it is the figure
    assert_eq!(runner_show(RADIUS), 1.0);
}

#[test]
fn feedback_keeps_a_figure_for_each_player_and_steps_it_at_that_players_speed() {
    let mut players = Players { list: Vec::new() };
    for slot in 0..3 {
        players.list.push(Player::viewing(slot, &format!("P{slot}")));
    }
    players.list[1].velocity = Vec2::new(0.0, PLAYER_SPEED);
    let mut feedback = Feedback::new();
    for _ in 0..60 {
        feedback.run(&players.list, DT);
    }
    assert_eq!(feedback.runners.len(), 3);
    assert_eq!(feedback.runners[0].run, 0.0);
    assert!(feedback.runners[1].run > 0.5 && feedback.runners[1].view == RunnerView::Front);
    //(a player who comes in later gets a figure, and one who leaves does not leave theirs behind)
    players.list.push(Player::viewing(3, "P3"));
    feedback.run(&players.list, DT);
    assert_eq!(feedback.runners.len(), 4);
    players.list.truncate(2);
    feedback.run(&players.list, DT);
    assert_eq!(feedback.runners.len(), 2, "it is not kept for the ones that are gone");
}

#[test]
fn the_name_above_a_player_clears_the_head_of_the_figure() {
    //the figure's head reaches RUNNER_TOP above the player's place at the very most, and the name is written above that
    let highest = [RunnerView::Front, RunnerView::Back, RunnerView::Side]
        .into_iter()
        .flat_map(|view| (0..64).flat_map(move |step| posed(view, step as f32 * std::f32::consts::TAU / 64.0, 1.0, 1.0).pose()))
        //(the head's radius and outline, and the 2 pixels the figure is drawn above the player's place)
        .filter_map(|part| if let RunnerPart::Head { at, .. } = part { Some(-at.y + 4.4 + 1.15 + 2.0) } else { None })
        .fold(0.0f32, f32::max);
    assert!(highest <= RUNNER_TOP, "{highest} against {RUNNER_TOP}");
    assert!(highest > RUNNER_TOP - 4.0, "and not by much more than it needs: {highest}");
}
