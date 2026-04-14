// Sprite batch shader — vertex and pixel shader in one file.
// Compile with:
//   fxc /T vs_4_0 /E vs_main sprite.hlsl /Fo sprite_vs.dxbc
//   fxc /T ps_4_0 /E ps_main sprite.hlsl /Fo sprite_ps.dxbc

cbuffer Transform : register(b0) {
    row_major float4x4 projection;
};

Texture2D    g_texture : register(t0);
SamplerState g_sampler : register(s0);

// ── Vertex shader ─────────────────────────────────────────────────────────────

struct VS_IN {
    float2 position : POSITION;
    float2 uv       : TEXCOORD0;
    float4 colour   : COLOR0;
};

struct VS_OUT {
    float4 position : SV_POSITION;
    float2 uv       : TEXCOORD0;
    float4 colour   : COLOR0;
};

VS_OUT vs_main(VS_IN input) {
    VS_OUT output;
    // Row-vector * row-major matrix: maps screen-space pos to clip-space.
    output.position = mul(float4(input.position, 0.0f, 1.0f), projection);
    output.uv       = input.uv;
    output.colour   = input.colour;
    return output;
}

// ── Pixel shader ──────────────────────────────────────────────────────────────

float4 ps_main(VS_OUT input) : SV_TARGET {
    return g_texture.Sample(g_sampler, input.uv) * input.colour;
}
