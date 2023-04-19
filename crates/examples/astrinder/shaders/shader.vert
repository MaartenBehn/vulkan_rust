#version 450

#define PART_COUNT 100

layout(location = 0) in vec3 vPosition;

layout(binding = 0) uniform RenderUniformBufferObject {
    vec4 cam;  // Pos X, Pos Y, Rot, Scale
    vec4 data; // Aspect
} render_ubo;

layout(binding = 1) uniform PartUniformBufferObject {
    vec4 data[PART_COUNT]; // Pos X, Pos Y, Rot
} part_ubo;

layout(binding = 2) buffer ParticleBuffer {
    uint data[];
} particle_buffer;


layout(location = 0) out flat uint index;
layout(location = 1) out vec2 pos;


vec2 rotate(vec2 v, float a) {
	float s = sin(a);
	float c = cos(a);
	mat2 m = mat2(c, -s, s, c);
	return m * v;
}

void main() {
    index = gl_InstanceIndex;
    pos = vPosition.xy;

    vec2 part_pos = part_ubo.data[index].xy;
    float part_rot = part_ubo.data[index].z;

    vec2 aspect = vec2(render_ubo.data.x, -1.0);
    vec2 vertex_pos = (rotate(vPosition.xy, part_rot) + part_pos - render_ubo.cam.xy) * aspect * render_ubo.cam.w; 

    gl_Position = vec4(vertex_pos, 0.0, 1.0);
}
