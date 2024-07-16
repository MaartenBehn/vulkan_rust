#version 450
#extension GL_ARB_shading_language_include : require

#include "../shader_includes/ray.glsl"
#include "../shader_includes/node.glsl"
#include "../shader_includes/debug.glsl"

// General
#define DEBUG_STEPS false
#define NODE_SIZE 4
#define MAX_STEPS 100
#define RAY_POS_OFFSET 0.0001
#define BORDER_SIZE 0.05

// Out
layout(location = 0) out vec4 finalColor;

// In
layout(location = 0) in vec3 oPos;
layout(location = 1) in vec3 oNormal;

// Render buffer
layout(set = 0, binding = 0) uniform RenderBuffer {
    mat4 proj_mat;
    mat4 view_mat;
    vec3 dir;
    vec2 size;
} renderbuffer;

// Voxels
layout(set = 0, binding = 1) buffer Nodes {
    Node nodes[];
} nodes;

// Materials 
layout(set = 0, binding = 2) buffer Mats {
    uint mats[];
} mats;

// Ship type (Push constant)
layout(push_constant, std430) uniform PushConstant {
    mat4 transform;
    uint data;
} push_constant;

// Chunk
layout(set = 1, binding = 0) buffer Chunk {
    uint node_ids[];
} chunk;


/*
let data = chunk_size_bits
*/
#define CHUNK_TRANSFORM push_constant.transform
#define CHUNK_SIZE      uint(1 << ((push_constant.data) & 15))  // 4 Bit


#define POSITION vec3(oPos * float(NODE_SIZE))
#define DIRECTION renderbuffer.dir
#define NORMAL oNormal

#define TO_NODE_ID_INDEX(pos, chunk_size) ((pos.z * chunk_size * chunk_size) + (pos.y * chunk_size) + pos.x)
#define GET_NODE_ID(index) chunk.node_ids[index]
#define GET_NODE(index) nodes.nodes[index]
#define GET_MAT(index) mats.mats[index]

// Debugging



bool hitBorder(in Ray ray){
    ivec3 pos = ivec3(ray.pos);
    vec3 lower = ray.pos - pos;
    vec3 upper = vec3(1.0) - lower;
    return (lower.x < BORDER_SIZE && lower.y < BORDER_SIZE) 
        || (lower.y < BORDER_SIZE && lower.z < BORDER_SIZE)
        || (lower.z < BORDER_SIZE && lower.x < BORDER_SIZE)
        || (upper.x < BORDER_SIZE && upper.y < BORDER_SIZE) 
        || (upper.y < BORDER_SIZE && upper.z < BORDER_SIZE)
        || (upper.z < BORDER_SIZE && upper.x < BORDER_SIZE)
        || (lower.x < BORDER_SIZE && upper.y < BORDER_SIZE)
        || (lower.y < BORDER_SIZE && upper.z < BORDER_SIZE)
        || (lower.z < BORDER_SIZE && upper.x < BORDER_SIZE)
        || (upper.x < BORDER_SIZE && lower.y < BORDER_SIZE)
        || (upper.y < BORDER_SIZE && lower.z < BORDER_SIZE)
        || (upper.z < BORDER_SIZE && lower.x < BORDER_SIZE);
}

vec4 voxelColor(in Ray ray, in uint voxel) {
    //return vec4(ray.pos, 1.0);

    if (hitBorder(ray)) {
            return vec4(0.0, 0.0, 0.0, 1.0);
        }

    uint mat = GET_MAT(voxel);
    return GET_MAT_VECTOR_FROM_MAT(mat);
}

vec4 raycaster(in Ray ray){
    float tMin, tMax = 0;
    float rayLen = 0;
    ivec3 node_size_half = ivec3(NODE_SIZE / 2);
    uint chunk_size = CHUNK_SIZE;
    uint chunk_voxel_size = chunk_size * NODE_SIZE;

    ivec3 cellPos;
    ivec3 nodePos;
    uint nodeID;
    Rot rot;
    uint nodeIndex;
    Node node;
    ivec3 voxelPos;
    uint voxelIndex;
    uint voxel;

    if (!aabb_ray_test(ray, vec3(0), vec3(1) * chunk_voxel_size, tMin, tMax)) {
        return vec4(0);
    }
    ray.pos += ray.dir * RAY_POS_OFFSET;
    //rayLen += RAY_POS_OFFSET;
    
    int counter = 1;
    while (counter < MAX_STEPS) {
        cellPos = ivec3(ray.pos);

        nodePos = cellPos / NODE_SIZE;
        nodeID = GET_NODE_ID(TO_NODE_ID_INDEX(nodePos, chunk_size));
        rot = GET_ROT_FROM_NODE_ID(nodeID);
        nodeIndex = GET_NODE_INDEX_FROM_NODE_ID(nodeID);

        node = GET_NODE(nodeIndex);

        voxelPos = ROTATE_VOXEL_POS(cellPos, rot);

        voxelIndex = GET_VOXEL_INDEX_FROM_VOXEL_POS(voxelPos);
        voxel = GET_VOXEL(node, voxelIndex);

        if (voxel != 0) {
            if (DEBUG_STEPS) {
                return vec4(get_debug_color_gradient_from_float(GET_GRADIENT(counter, MAX_STEPS)), 1.0);
            }

            return voxelColor(ray, voxel);
        }

        aabb_ray_test(ray, vec3(cellPos), vec3(cellPos) + 1, tMin, tMax);
        ray.pos = ray.pos + ray.dir * (tMax + RAY_POS_OFFSET);
        rayLen += tMax;

        if (ray.pos.x < 0 || ray.pos.x >= chunk_voxel_size || ray.pos.y < 0 || ray.pos.y >= chunk_voxel_size || ray.pos.z < 0 || ray.pos.z >= chunk_voxel_size){
            break;
        }

        counter++;
    }

    if (DEBUG_STEPS) {
        return vec4(get_debug_color_gradient_from_float(GET_GRADIENT(counter, MAX_STEPS)), 1.0);
    }
    return vec4(0);
}

void main() {
    Ray ray = init_ray(POSITION, DIRECTION, gl_FragCoord.xy, renderbuffer.size.xy);
    float tMin, tMax;
    vec4 color = raycaster(ray);

    finalColor = color;

    if (color.w == 0) {
        gl_FragDepth = 1.0;
    } else {
        gl_FragDepth = gl_FragCoord.z;
    }
}
