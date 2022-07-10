fn alpha_blend(col: vec4<f32>, alpha: f32) -> vec4<f32> {
    return vec4<f32>(
        col[0],
        col[1],
        col[2],
        col[3] * alpha,
    );
}
