#version 450

#define PART_COUNT 100
#define CHUNK_SIZE 10


#define MAX_16_BIT 65535

layout(location = 0) in flat uint index;
layout(location = 1) in vec2 pos;

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

#define GET_PARTICLE_INDEX(pos, instance) instance * CHUNK_SIZE * CHUNK_SIZE + pos.x * CHUNK_SIZE + pos.y
#define GET_PARTICLE_MATERIAL(pos, instance) particle_buffer.data[GET_PARTICLE_INDEX(pos, instance)] & MAX_16_BIT

layout(location = 0) out vec4 finalColor;

uvec2 coord_to_hex(vec2 coord) {
    return uvec2(
        (coord.x - coord.y * 0.5),
        (coord.y)
    );
}

vec2 hex_to_coord(uvec2 hex) {
    return vec2(
        (hex.x + 0.5) + (hex.y + 0.5) * 0.5, 
        hex.y + 0.5
    );
}

void main() {
    uvec2 particle_pos = coord_to_hex(pos);

    float dist = length(hex_to_coord(particle_pos) - pos);

    uint material = GET_PARTICLE_MATERIAL(particle_pos, index);

    float draw = float(dist < 0.4 && material != 0); 

    finalColor = vec4(vec3(1.0 - draw), 1.0);
}
