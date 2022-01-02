#include <metal_stdlib>

using namespace metal;

struct RasterizerData
{
    // The [[position]] attribute of this member indicates that this value
    // is the clip space position of the vertex when this structure is
    // returned from the vertex function.
    float4 position [[position]];

    // Since this member does not have a special attribute, the rasterizer
    // interpolates its value with the values of the other triangle vertices
    // and then passes the interpolated value to the fragment shader for each
    // fragment in the triangle.
    float4 color;
};

struct Vertex
{
	packed_float3 position;
	packed_float4 color;
};

struct VertexUniforms
{
    float4x4 mvp_matrix;
};

vertex RasterizerData triangle_vertex(
    uint vid [[vertex_id]], constant VertexUniforms& uniforms [[buffer(0)]], const device Vertex* vertices [[buffer(1)]])
{
    RasterizerData outData;
    auto device const &v = vertices[vid];
    outData.position = float4(v.position.x, v.position.y, v.position.z, 1.0) * uniforms.mvp_matrix;
    outData.color = v.color;

    return outData;
}

// fragment shader function
fragment float4 triangle_fragment(RasterizerData in [[stage_in]])
{
    return in.color;
};