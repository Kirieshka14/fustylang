// 2D & 3D vector physics demo in FustyLang

let mut pos = vec2(0.0, 10.0);
let mut vel = vec2(4.0, 0.0);
let gravity = vec2(0.0, -9.81);
let dt = 0.016;

let mut step = 0;
while step < 5 {
    vel += gravity * dt;
    pos += vel * dt;
    step += 1;
}

log("2D pos after 5 steps =", pos, "vel =", vel);

let a = vec3(1.0, 0.0, 0.0);
let b = vec3(0.0, 1.0, 0.0);
let c = a.cross(b);
log("3D cross(a, b) =", c, "length =", c.length());
