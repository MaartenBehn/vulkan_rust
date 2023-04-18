#version 450

#define PART_COUNT 100

layout(location = 0) in vec3 vPosition;

layout(binding = 0) uniform RenderUniformBufferObject {
    vec4 cam;  // Pos X, Pos Y, Rot, Scale
    vec4 data; // Aspect
} render_ubo;

layout(binding = 1) uniform PartUniformBufferObject {
    vec4 data[PART_COUNT];
} part_ubo;

layout(binding = 2) buffer ParticleBuffer {
    uint data[];
} particle_buffer;


layout(location = 0) out flat uint index;
layout(location = 1) out vec2 pos;

void main() {
    index = gl_InstanceIndex;
    pos = vPosition.xy;

    vec2 part_pos = part_ubo.data[index].xy;
    vec2 aspect = vec2(render_ubo.data.x, -1.0);
    vec2 vertex_pos = (vPosition.xy + part_pos - render_ubo.cam.xy) * aspect * render_ubo.cam.w; 

    gl_Position = vec4(vertex_pos, 0.0, 1.0);
}
