#include <metal_stdlib>
#include <MetalPerformanceShaders/MetalPerformanceShaders.h>

using namespace metal;

struct RasterizerData
{
    // The [[position]] attribute of this member indicates that this value
    // is the clip space position of the vertex when this structure is
    // returned from the vertex function.
    float4 position [[position]];
    float3 normal;
};

struct Vertex
{
	packed_float3 position;
};

struct VertexUniforms
{
    float4x4 mvp_matrix;
};

vertex RasterizerData triangle_vertex(
    uint vid [[vertex_id]],
    constant VertexUniforms& uniforms [[buffer(0)]],
    const device packed_float3* vertices [[buffer(1)]],
    const device packed_float3* normals [[buffer(2)]])
{
    RasterizerData outData;
    auto device const &position = vertices[vid];
    outData.position = float4(position.x, position.y, position.z, 1.0) * uniforms.mvp_matrix;
    auto device const &normal = normals[vid];
    outData.normal = (float4(normal.x, normal.y, normal.z, 1.0) * uniforms.mvp_matrix).xyz;

    return outData;
}

// fragment shader function
fragment float4 triangle_fragment(RasterizerData in [[stage_in]], float3 bary [[barycentric_coord]])
{
    return float4(bary.x, bary.y, bary.z, 1.0);
};


using Ray = MPSRayOriginMinDistanceDirectionMaxDistance;
using Intersection = MPSIntersectionDistancePrimitiveIndexCoordinates;

kernel void generateRays(
    device Ray* rays [[buffer(0)]],
    uint2 coordinates [[thread_position_in_grid]],
    uint2 size [[threads_per_grid]])
{
    float2 uv = float2(coordinates) / float2(size - 1);

    uint rayIndex = coordinates.x + coordinates.y * size.x;
    rays[rayIndex].origin = MPSPackedFloat3(uv.x, uv.y, -1.0);
    rays[rayIndex].direction = MPSPackedFloat3(0.0, 0.0, 1.0);
    rays[rayIndex].minDistance = 0.0f;
    rays[rayIndex].maxDistance = 2.0f;
}