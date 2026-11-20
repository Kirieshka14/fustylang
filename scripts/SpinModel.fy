// Ferrium FustyLang — 3D/2D Model Animation Script (SpinModel.fy)
// Attach to any 3D/2D model in Inspector and press Play!

let spin_speed = 1.6;
let bob_amp = 0.25;

fn animate_model() {
    let t = time();
    set_self_rot(vec3(0.0, t * spin_speed, sin(t) * 0.25));
    let mut p = get_self_pos3();
    p.y = abs(sin(t * 2.0)) * bob_amp;
    set_self_pos3(p);
}

while true {
    animate_model();
    wait 0.016;
}
