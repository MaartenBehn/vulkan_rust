#version 450

layout(location = 0) in uint vData;
layout(location = 0) out vec3 oNormal;

layout(binding = 0) uniform RenderBuffer {
    mat4 proj_mat;
    mat4 view_mat;
    vec3 dir;
    vec2 size;
} ubo;

/*
let data = ((pos.x & 0b1111)
            + ((pos.y & 0b1111) << 4)
            + ((pos.z & 0b1111) << 8)
            + (((normal.x == 1) as u32) << 12)
            + (((normal.y == 1) as u32) << 13)
            + (((normal.z == 1) as u32) << 14)) as u16;
*/

void main() {
    vec4 p = vec4(
        float(vData & 15),
        float((vData >> 4) & 15),
        float((vData >> 8) & 15), 1.0);

    gl_Position = ubo.proj_mat * ubo.view_mat * p;

    oNormal = vec3(
        float((vData >> 12) & 1),
        float((vData >> 13) & 1),
        float((vData >> 14) & 1));
}
