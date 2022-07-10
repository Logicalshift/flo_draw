fn alpha_blend(col: vec4<f32>, alpha: f32) -> vec4<f32> {
    return vec4<f32>(
        col[0] * alpha,
        col[1] * alpha,
        col[2] * alpha,
        col[3] * alpha,
    );
}
