typedef struct {
    float4 v_Position [[position]];
    float4 v_Color;
    float2 v_TexCoord;
    float2 v_PaperCoord;
} RasterizerData;

float4 apply_clip_mask(
      float4                    color, 
      float2                    paper_coord,
      metal::texture2d_ms<half> eraser_texture);

float4 invert_color_alpha(float4 col);
float4 multiply_alpha(float4 col);
