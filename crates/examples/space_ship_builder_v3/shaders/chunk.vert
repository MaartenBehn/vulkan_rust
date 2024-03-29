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
        float(vData & 511),
        float((vData >> 9) & 511),
        float((vData >> 18) & 511));

    oPos = p;
    oNormal = vec3(
        float((vData >> 27) & 1),
        float((vData >> 28) & 1),
        float((vData >> 29) & 1));

    gl_Position = renderbuffer.proj_mat * renderbuffer.view_mat * vec4(p, 1.0);
}
