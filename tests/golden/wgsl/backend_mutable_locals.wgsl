fn counter() -> u32 {
    var count = 0;
    count = (count + 1);
    return count;
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) id: vec3<u32>) {
    var data: array<u32, 10> = /* TODO */;
    data[id.x] = 42;
}

