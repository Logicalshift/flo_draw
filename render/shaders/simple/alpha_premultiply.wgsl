fn color_process_alpha(col: vec<f32>) -> vec<f32> {
    let new_col = vec<f32>(
        col[0] * col[3],
        col[1] * col[3],
        col[2] * col[3],
        col[3]
    );

    return new_col;
}
