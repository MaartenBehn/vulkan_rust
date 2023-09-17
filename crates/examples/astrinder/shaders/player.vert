#version 450

#define PART_COUNT 100

layout(location = 0) in vec3 vPosition;

layout(binding = 0) uniform RenderUniformBufferObject {
    vec4 cam;  // Pos X, Pos Y, Rot, Scale
    vec4 data; // Aspect
} render_ubo;

void main() {
    vec2 aspect = vec2(render_ubo.data.x, -1.0);
    vec2 vertex_pos = (vPosition.xy - render_ubo.cam.xy) * aspect * render_ubo.cam.w; 

    gl_Position = vec4(vertex_pos, 0.0, 1.0);
}
