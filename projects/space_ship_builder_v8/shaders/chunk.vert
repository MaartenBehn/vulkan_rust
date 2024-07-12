#version 450

layout(location = 0) in uint vData;
layout(location = 0) out vec3 oPos;
layout(location = 1) out vec3 oNormal;

layout(set = 0, binding = 0) uniform RenderBuffer {
    mat4 proj_mat;
    mat4 view_mat;
    vec3 dir;
    vec2 size;
} renderbuffer;

layout(push_constant, std430) uniform PushConstant {
    mat4 transform;
    uint data;
} push_constant;

/*
let data = chunk_size_bits
*/
#define CHUNK_TRANSFORM push_constant.transform
#define CHUNK_SIZE      uint(1 << ((push_constant.data) & 15))  // 4 Bit

void main() {
    vec3 p = vec3(
        float(vData & uint(511)),
        float((vData >> 9) & uint(511)),
        float((vData >> 18) & uint(511)));

    oPos = p;
    oNormal = vec3(
        float((vData >> 27) & uint(1)),
        float((vData >> 28) & uint(1)),
        float((vData >> 29) & uint(1)));

    gl_Position = renderbuffer.proj_mat * renderbuffer.view_mat * CHUNK_TRANSFORM * vec4(p, 1.0);
}
