// The `uwu` operator: cuddle two values into their midpoint.
purr("1 uwu 2 =", 1 uwu 2);

let a = vec2(0.0, 0.0);
let b = vec2(10.0, 20.0);
purr("vec2 cuddle =", a uwu b);
purr("left-assoc  =", a uwu b uwu b);
