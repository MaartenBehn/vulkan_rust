#version 450

#define PART_COUNT 100

layout(location = 0) in vec3 vPosition;

layout(binding = 0) uniform RenderUniformBufferObject {
    vec4 cam;  // Pos X, Pos Y, Rot, Scale
    vec4 data; // Aspect
} cam_ubo;

void main() {
    vec2 aspect = vec2(cam_ubo.data.x, -1.0);
    vec2 vertex_pos = (vPosition.xy - cam_ubo.cam.xy) * aspect * cam_ubo.cam.w; 

    gl_Position = vec4(vertex_pos, 0.0, 1.0);
}
