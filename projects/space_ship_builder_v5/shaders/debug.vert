#version 450

layout(location = 0) in vec4 vData;

layout(location = 0) out vec4 oColor;

layout(binding = 0) uniform RenderBuffer {
    mat4 proj_mat;
    mat4 view_mat;
    vec3 dir;
} ubo;

#define POSITION vData.xyz

void main() {
    uint data = floatBitsToUint(vData.w);
    vec4 color = vec4(255.0 / (data & 7), 255.0 / ((data >> 8) & 7), 255.0 / ((data >> 16) & 7), 255.0 / ((data >> 24) & 7));

    gl_Position = ubo.proj_mat * ubo.view_mat * vec4(POSITION, 1.0);
    oColor = color;
}
