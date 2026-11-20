// Ferrium FustyLang — Player Controller (Player.fy)

fn move_player(dir: vec2, speed: f64) {
    set_self_vel(dir * speed);
}

fn update() {
    let speed = 5.0;
    let axis = vec2(input_x(), input_y());
    if axis.length() > 0.01 {
        move_player(axis.normalized(), speed);
    } else {
        set_self_vel(vec2(0.0, 0.0));
    }
}

while true {
    update();
    wait 0.016;
}
