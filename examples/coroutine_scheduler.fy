// Multi-step coroutine scheduler demo using wait and yield

nya patrol_step(id: i64) {
    log("agent", id, "starting patrol");
    wait 0.05;
    log("agent", id, "checkpoint A reached");
    yield;
    log("agent", id, "checkpoint B reached");
}

patrol_step(1);
log("patrol_step finished");
