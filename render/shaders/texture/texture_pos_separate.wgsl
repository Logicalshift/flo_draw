fn texture_position(pos: vec2<f32>, tex_coord: vec2<f32>, texture_transform: mat4x4<f32>) -> vec2<f32> {
    let tex_pos = vec4<f32>(tex_coord[0], tex_coord[1], 0.0, 1.0) * texture_transform;

    return vec2<f32>(tex_pos[0], tex_pos[1]);
}
