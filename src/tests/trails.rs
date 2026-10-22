//particle trails

use super::*;


// ---------- particle trails ----------

#[test]
fn trail_particles_start_behind_a_moving_circle_and_fly_backwards() {
    let position = Vec2::new(3000.0, 3000.0);
    for angle in [0.0f32, 0.7, 1.9, 3.14, 4.4, 5.8] {
        let heading = Vec2::new(angle.cos(), angle.sin());
        let mut particles = Vec::new();
        // lots of frames so there are plenty of particles to check
        for _ in 0..200 {
            emit_trail(&mut particles, position, heading * 400.0, 400.0, 1.0 / 120.0, RED);
        }
        assert!(particles.len() > 50, "only {} particles", particles.len());
        for particle in &particles {
            assert!((particle.position - position).dot(heading) < 0.0, "starts in front of the circle, heading {angle}");
            assert!(particle.velocity.dot(heading) < 0.0, "flies forwards, heading {angle}");
            // and mostly backwards, not sideways
            assert!(particle.velocity.dot(-heading) > particle.velocity.dot(heading.perp()).abs());
        }
    }
}

#[test]
fn a_circle_that_is_hardly_moving_leaves_no_trail() {
    let mut particles = Vec::new();
    for _ in 0..1000 {
        emit_trail(&mut particles, Vec2::new(500.0, 500.0), Vec2::ZERO, 400.0, 1.0 / 120.0, RED);
        emit_trail(&mut particles, Vec2::new(500.0, 500.0), Vec2::new(400.0 * TRAIL_MIN_SPEED * 0.9, 0.0), 400.0, 1.0 / 120.0, RED);
    }
    assert!(particles.is_empty());
    emit_trail(&mut particles, Vec2::new(500.0, 500.0), Vec2::new(0.0, 400.0 * TRAIL_MIN_SPEED * 1.1), 400.0, 1.0, RED);
    assert!(!particles.is_empty());
}

#[test]
fn the_trail_is_as_thick_at_any_frame_rate() {
    for fps in [30.0f32, 60.0, 120.0, 240.0, 500.0] {
        let dt = 1.0 / fps;
        let mut particles = Vec::new();
        let mut made = 0;
        for _ in 0..(fps * 20.0) as usize {
            let before = particles.len();
            emit_trail(&mut particles, Vec2::new(900.0, 900.0), Vec2::new(300.0, 0.0), 300.0, dt, RED);
            made += particles.len() - before;
            update_particles(&mut particles, dt);
        }
        let expected = 300.0 * 20.0 / TRAIL_SPACING;
        println!("{fps} fps: {made} particles in 20 s, expected {expected}");
        assert!((made as f32 - expected).abs() < expected * 0.2, "{fps} fps made {made}");
    }
}

#[test]
fn there_are_never_more_than_the_particle_limit_and_a_crowd_shares_them_out() {
    // 3000 circles on screen all moving: far more particles wanted than the limit allows
    let mut particles = Vec::new();
    let mut made = vec![0usize; 3000];
    let dt = 1.0 / 120.0;
    let mut most = 0;
    for frame in 0..(120 * 6) {
        for count in made.iter_mut() {
            let before = particles.len();
            emit_trail(&mut particles, Vec2::new(900.0, 900.0), Vec2::new(300.0, 0.0), 300.0, dt, RED);
            if frame >= 120 * 3 {
                *count += particles.len() - before;
            }
        }
        update_particles(&mut particles, dt);
        most = most.max(particles.len());
    }
    println!("most particles at once {most} of {MAX_PARTICLES}");
    assert!(most <= MAX_PARTICLES);
    // the first circles in the list don't get all of them: compare the first and last thousand
    let first: usize = made[..1000].iter().sum();
    let last: usize = made[2000..].iter().sum();
    println!("first thousand circles made {first}, last thousand {last}");
    assert!(first > 0 && last > 0);
    assert!((first as f32) < last as f32 * 1.5 && (last as f32) < first as f32 * 1.5, "{first} vs {last}");
}

#[test]
fn particles_drift_back_slow_down_and_die() {
    let mut particles = Vec::new();
    emit_trail(&mut particles, Vec2::new(2000.0, 2000.0), Vec2::new(500.0, 0.0), 500.0, 1.0, RED);
    assert!(particles.len() > 10);
    let start: Vec<Vec2> = particles.iter().map(|particle| particle.position).collect();
    let dt = 1.0 / 120.0;
    let mut speeds = Vec::new();
    for _ in 0..30 {
        update_particles(&mut particles, dt);
        speeds.push(particles.iter().map(|particle| particle.velocity.length()).sum::<f32>() / particles.len().max(1) as f32);
    }
    // the ones still alive have moved backwards (-x) and slowed down
    assert!(!particles.is_empty());
    assert!(particles.iter().map(|p| p.position.x).sum::<f32>() / (particles.len() as f32) < start.iter().map(|p| p.x).sum::<f32>() / start.len() as f32);
    assert!(speeds.last().unwrap() < speeds.first().unwrap());
    // all gone after the longest life
    for _ in 0..((TRAIL_LIFE + 0.1) * 120.0) as usize {
        update_particles(&mut particles, dt);
    }
    assert!(particles.is_empty());
}

#[test]
fn smoothed_velocity_follows_steady_movement_and_ignores_jitter() {
    let dt = 1.0 / 120.0;
    // steady 400 px/s to the right
    let mut velocity = Vec2::ZERO;
    for frame in 0..60 {
        let from = Vec2::new(frame as f32 * 400.0 * dt, 0.0);
        velocity = smoothed_velocity(velocity, from, from + Vec2::new(400.0 * dt, 0.0), dt);
    }
    assert!((velocity - Vec2::new(400.0, 0.0)).length() < 5.0, "{velocity}");
    // a circle shoved 1.5 px one way then the other every frame goes nowhere
    let mut velocity = Vec2::ZERO;
    let mut biggest = 0.0f32;
    for frame in 0..600 {
        let shove = if frame % 2 == 0 { 1.5 } else { -1.5 };
        velocity = smoothed_velocity(velocity, Vec2::ZERO, Vec2::new(shove, shove * 0.5), dt);
        if frame > 60 {
            biggest = biggest.max(velocity.length());
        }
    }
    println!("jitter of 1.5 px a frame gives a velocity of at most {biggest:.1} px/s (raw {:.0})", 1.5 / dt);
    assert!(biggest < UNIT_MIN_SPEED * TRAIL_MIN_SPEED, "{biggest}");
    // no time passing leaves it as it was
    assert_eq!(smoothed_velocity(Vec2::new(3.0, 4.0), Vec2::ZERO, Vec2::new(99.0, 99.0), 0.0), Vec2::new(3.0, 4.0));
}

#[test]
fn a_walking_unit_has_the_velocity_it_walks_at_and_a_trail_behind_it() {
    let walls = Walls::default();
    let player = Vec2::new(5000.0, 3000.0);
    let field = field_for(player, &walls);
    let mut units = vec![Unit::new(2000.0, 3000.0, 300.0)];
    let dt = 1.0 / 120.0;
    for _ in 0..120 {
        update_units(&mut units, player.x, player.y, dt, &walls, &field, 1.0, &Squads::new());
    }
    let velocity = units[0].velocity;
    println!("unit velocity {velocity}");
    assert!((velocity - Vec2::new(300.0, 0.0)).length() < 10.0, "{velocity}");
    let mut particles = Vec::new();
    for _ in 0..120 {
        emit_trail(&mut particles, Vec2::new(units[0].x, units[0].y), velocity, 300.0, dt, RED);
    }
    assert!(particles.iter().all(|p| p.velocity.x < 0.0));
}

#[test]
fn units_stuck_behind_the_house_leave_hardly_any_trail() {
    let walls = starting_house();
    let player = Vec2::new(WORLD_SIZE / 2.0, WORLD_SIZE / 2.0);
    let field = field_for(player, &walls);
    let mut units = Vec::new();
    for _ in 0..400 {
        let position = far_position(player.x, player.y);
        units.push(Unit::new(position.x, position.y, rand::gen_range(UNIT_MIN_SPEED, UNIT_MAX_SPEED)));
    }
    let dt = 1.0 / 120.0;
    let mut moving_while_approaching = 0;
    for frame in 0..(120 * 40) {
        update_units(&mut units, player.x, player.y, dt, &walls, &field, 1.0, &Squads::new());
        if frame == 120 * 3 {
            moving_while_approaching = units.iter().filter(|e| e.velocity.length() >= e.speed * TRAIL_MIN_SPEED).count();
        }
    }
    // where the piled up units' velocity comes from: net movement over the next second against what they jitter by
    let mut speeds: Vec<f32> = units.iter().map(|e| e.velocity.length()).collect();
    speeds.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let before: Vec<Vec2> = units.iter().map(|e| Vec2::new(e.x, e.y)).collect();
    let mut path = vec![0.0f32; units.len()];
    for _ in 0..120 {
        let last: Vec<Vec2> = units.iter().map(|e| Vec2::new(e.x, e.y)).collect();
        update_units(&mut units, player.x, player.y, dt, &walls, &field, 1.0, &Squads::new());
        for (i, e) in units.iter().enumerate() {
            path[i] += Vec2::new(e.x, e.y).distance(last[i]);
        }
    }
    let net: Vec<f32> = units.iter().zip(&before).map(|(e, b)| Vec2::new(e.x, e.y).distance(*b)).collect();
    println!(
        "piled up: smoothed speed median {:.0}, p90 {:.0}; over the next second the median unit walked {:.0} px but ended {:.0} px from where it started",
        speeds[200], speeds[360],
        { let mut p = path.clone(); p.sort_by(|a, b| a.partial_cmp(b).unwrap()); p[200] },
        { let mut n = net.clone(); n.sort_by(|a, b| a.partial_cmp(b).unwrap()); n[200] },
    );
    let still_trailing = units.iter().filter(|e| e.velocity.length() >= e.speed * TRAIL_MIN_SPEED).count();
    println!("moving at 3 s: {moving_while_approaching} of 400, still leaving a trail after 40 s: {still_trailing}");
    assert!(moving_while_approaching > 250);
    assert!(still_trailing < 80, "{still_trailing} units still leave trails after piling up");
}
