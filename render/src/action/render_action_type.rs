use super::render_action::*;

///
/// An enumeration of the types of possible render actions without their data (useful for logging and profiling)
///
#[derive(Copy, Clone, PartialEq, Debug, Eq, Hash)]
pub enum RenderActionType {
    SetTransform,
    CreateVertex2DBuffer,
    CreateIndexBuffer,
    FreeVertexBuffer,
    FreeIndexBuffer,
    BlendMode,
    CreateRenderTarget,
    FreeRenderTarget,
    SelectRenderTarget,
    RenderToFrameBuffer,
    ShowFrameBuffer,
    DrawFrameBuffer,
    CreateTextureBgra,
    CreateTextureMono,
    Create1DTextureBgra,
    Create1DTextureMono,
    WriteTextureData,
    WriteTexture1D,
    CreateMipMaps,
    CopyTexture,
    FilterTexture,
    FreeTexture,
    Clear,
    UseShader,
    DrawTriangles,
    DrawIndexedTriangles,
}

impl From<&RenderAction> for RenderActionType {
    fn from(render_action: &RenderAction) -> RenderActionType {
        match render_action {
            RenderAction::SetTransform(_)                   => RenderActionType::SetTransform,
            RenderAction::CreateVertex2DBuffer(_, _)        => RenderActionType::CreateVertex2DBuffer,
            RenderAction::CreateIndexBuffer(_, _)           => RenderActionType::CreateIndexBuffer,
            RenderAction::FreeVertexBuffer(_)               => RenderActionType::FreeVertexBuffer,
            RenderAction::FreeIndexBuffer(_)                => RenderActionType::FreeIndexBuffer,
            RenderAction::BlendMode(_)                      => RenderActionType::BlendMode,
            RenderAction::CreateRenderTarget(_, _, _, _)    => RenderActionType::CreateRenderTarget,
            RenderAction::FreeRenderTarget(_)               => RenderActionType::FreeRenderTarget,
            RenderAction::SelectRenderTarget(_)             => RenderActionType::SelectRenderTarget,
            RenderAction::RenderToFrameBuffer               => RenderActionType::RenderToFrameBuffer,
            RenderAction::ShowFrameBuffer                   => RenderActionType::ShowFrameBuffer,
            RenderAction::DrawFrameBuffer(_, _, _)          => RenderActionType::DrawFrameBuffer,
            RenderAction::CreateTextureBgra(_, _)           => RenderActionType::CreateTextureBgra,
            RenderAction::CreateTextureMono(_, _)           => RenderActionType::CreateTextureMono,
            RenderAction::Create1DTextureBgra(_, _)         => RenderActionType::Create1DTextureBgra,
            RenderAction::Create1DTextureMono(_, _)         => RenderActionType::Create1DTextureMono,
            RenderAction::WriteTextureData(_, _, _, _)      => RenderActionType::WriteTextureData,
            RenderAction::WriteTexture1D(_, _, _, _)        => RenderActionType::WriteTexture1D,
            RenderAction::CreateMipMaps(_)                  => RenderActionType::CreateMipMaps,
            RenderAction::CopyTexture(_, _)                 => RenderActionType::CopyTexture,
            RenderAction::FilterTexture(_, _)               => RenderActionType::FilterTexture,
            RenderAction::FreeTexture(_)                    => RenderActionType::FreeTexture,
            RenderAction::Clear(_)                          => RenderActionType::Clear,
            RenderAction::UseShader(_)                      => RenderActionType::UseShader,
            RenderAction::DrawTriangles(_, _)               => RenderActionType::DrawTriangles,
            RenderAction::DrawIndexedTriangles(_, _, _)     => RenderActionType::DrawIndexedTriangles,
        }
    }
}