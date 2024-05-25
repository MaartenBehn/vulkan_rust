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
    uint data;
} push_constant;

#define CHUNK_POS        ivec3(int(push_constant.data & 255) - 128, int((push_constant.data >> 8) & 255) - 128, int((push_constant.data >> 16) & 255) - 128)
#define CHUNK_SIZE       int(1 << ((push_constant.data >> 24) & 15))  // 4 Bit

/*
 let data = (pos.x & 0b111111111)
            + ((pos.y & 0b111111111) << 9)
            + ((pos.z & 0b111111111) << 18)
            + (((normal.x == 1) as u32) << 27)
            + (((normal.y == 1) as u32) << 28)
            + (((normal.z == 1) as u32) << 29);
*/

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

    gl_Position = renderbuffer.proj_mat * renderbuffer.view_mat * vec4(p + vec3(CHUNK_POS * CHUNK_SIZE), 1.0);
}
