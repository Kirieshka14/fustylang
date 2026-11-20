// ============================================================================
// Ferrium Engine — Live Hot-Reloadable Boss AI Script (FustyLang .fy)
// Edit & save this file while `ferrium_engine` is running to hot-reload live!
// ============================================================================

fn dash_strike(dir: vec2, speed: f64) {
    set_boss_state("windup");
    set_self_vel(vec2(0.0, 0.0));
    spawn_particles(get_self_pos(), 8, 252, 186, 48);
    wait 0.28;

    set_boss_state("rush");
    set_self_vel(dir * speed);
    camera_shake(4.0, 0.12);
    spawn_hitbox(dir * 26.0, vec2(68.0, 68.0), 18, 0.24);
    wait 0.24;

    set_self_vel(vec2(0.0, 0.0));
    set_boss_state("recover");
    wait 0.22;
}

log("Boss AI online (FustyLang coroutine)");

while true {
    let my_pos = get_self_pos();
    let target = get_player_pos();
    let hp = get_self_hp();

    if hp <= 0 {
        set_boss_state("defeated");
        set_self_vel(vec2(0.0, 0.0));
        wait 0.50;
    } else {
        let delta = target - my_pos;
        let dist = delta.length();
        let dir = delta.normalized();

        if hp <= 500 {
            // Phase 2: Overdrive with meow raw_teleport blink-strike!
            set_boss_state("enrage");
            spawn_particles(my_pos, 10, 235, 75, 225);
            wait 0.22;

            let blink_pos = target - dir * 72.0;
            meow {
                raw_teleport(blink_pos);
            }
            spawn_particles(get_self_pos(), 16, 235, 75, 225);
            camera_shake(9.0, 0.22);
            log("MEOW BLINK STRIKE!");

            wait 0.16;
            spawn_hitbox(vec2(0.0, 0.0), vec2(128.0, 128.0), 24, 0.24);
            wait 0.30;

            let rush_dir = (get_player_pos() - get_self_pos()).normalized();
            dash_strike(rush_dir, 540.0);
        } else if dist > 210.0 {
            // Phase 1: Stalking pursuit
            set_boss_state("chase");
            set_self_vel(dir * 165.0);
            wait 0.14;
        } else if dist > 95.0 {
            // Phase 1: Telegraph & rush combo
            dash_strike(dir, 430.0);
        } else {
            // Close-range sweeping cleave
            set_boss_state("windup");
            set_self_vel(vec2(0.0, 0.0));
            spawn_particles(my_pos, 6, 252, 186, 48);
            wait 0.22;

            set_boss_state("slash");
            let strike_dir = (get_player_pos() - get_self_pos()).normalized();
            spawn_hitbox(strike_dir * 42.0, vec2(84.0, 84.0), 16, 0.20);
            camera_shake(5.5, 0.14);
            wait 0.34;
        }
    }
}
