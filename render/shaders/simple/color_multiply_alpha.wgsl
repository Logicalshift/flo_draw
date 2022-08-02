fn color_post_process(col: vec4<f32>) -> vec4<f32> {
    let new_col = vec4<f32>(
        col[0] * col[3],
        col[1] * col[3],
        col[2] * col[3],
        col[3]
     );

    return new_col;
}
