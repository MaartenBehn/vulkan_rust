#version 450

layout(location = 0) in vec4 vData;

layout(location = 0) out vec3 oUV;
layout(location = 1) flat out uint oNodeId;

layout(binding = 0) uniform RenderBuffer {
    mat4 proj_mat;
    mat4 view_mat;
    vec3 dir;
} ubo;

#define POSITION vData.xyz

void getData(out vec3 uv, out uint node_id) {
    
}

void main() {
    uint data = floatBitsToUint(vData.w);
    oUV = vec3(uint((data & 4) != 0), uint((data & 2) != 0), uint((data & 1) != 0));
    oNodeId = data >> 3;

    gl_Position = ubo.proj_mat * ubo.view_mat * vec4(POSITION, 1.0);
}
