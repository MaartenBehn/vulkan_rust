#version 450

// General
#define DEBUG_STEPS true
#define NODE_SIZE 4
#define CHUNK_SIZE 16
#define MAX_STEPS 100
#define RAY_POS_OFFSET 0.0001
#define BORDER_SIZE 0.01

// Out
layout(location = 0) out vec4 finalColor;


// In
layout(location = 0) in vec3 oPos;
#define POSITION oPos * NODE_SIZE

layout(location = 1) in vec3 oNormal;
#define NORMAL oNormal

// Render buffer
layout(set = 0, binding = 0) uniform RenderBuffer {
    mat4 proj_mat;
    mat4 view_mat;
    vec3 dir;
    vec2 size;
} renderbuffer;
#define DIRECTION renderbuffer.dir

// Voxels
struct Node {
    uint voxels[(NODE_SIZE * NODE_SIZE * NODE_SIZE) / 4];
};

layout(set = 0, binding = 1) buffer Nodes {
    Node nodes[];
} nodes;
#define TO_VOXEL_INDEX(pos) ((pos.z * NODE_SIZE * NODE_SIZE) + (pos.y * NODE_SIZE) + pos.x)
#define GET_VOXEL(node, index) (node.voxels[index / 4] >> ((index % 4) * 8)) & 255
#define GET_NODE(index) nodes.nodes[index]


// Materials 
layout(set = 0, binding = 2) buffer Mats {
    uint mats[];
} mats;
#define GET_MAT(mat) (vec4(float(mat & 255) / 255.0, float((mat >> 8) & 255) / 255.0, float((mat >> 16) & 255) / 255.0, float((mat >> 24) & 255) / 255.0))


// Ship type (Push constant)
layout(push_constant, std430) uniform PushConstant {
    uint ship_type;
} push_constant;
#define SHIP_TYPE push_constant.ship_type


// Chunk
layout(set = 1, binding = 0) buffer Chunk {
    uint node_ids[];
} chunk;
#define TO_NODE_ID_INDEX(pos) ((pos.z * CHUNK_SIZE * CHUNK_SIZE) + (pos.y * CHUNK_SIZE) + pos.x)
#define GET_NODE_ID(index) chunk.node_ids[index]


struct Rot {
    mat4 mat;
    ivec3 offset;
};
Rot GET_ROT_FROM_NODE_ID(uint nodeID) {
    uint index_nz1 = nodeID & 3;
    uint index_nz2 = (nodeID >> 2) & 3;
    uint index_nz3 = 3 - index_nz1 - index_nz2;

    int row_1_sign = (nodeID & (1 << 4)) == 0 ? 1 : -1;
    int row_2_sign = (nodeID & (1 << 5)) == 0 ? 1 : -1;
    int row_3_sign = (nodeID & (1 << 6)) == 0 ? 1 : -1;

    mat4 mat = mat4(0);
    mat[index_nz1][0] = row_1_sign;
    mat[index_nz2][1] = row_2_sign;
    mat[index_nz3][2] = row_3_sign;

    Rot rot = Rot(mat, ivec3(row_1_sign == -1, row_2_sign == -1, row_3_sign == -1));
    return rot;
}
#define GET_NODE_INDEX_FROM_NODE_ID(nodeID) nodeID >> 7


// Debugging
vec3 getColorGradient(float x){
    if (x == 0){
        return vec3(0);
    }

    vec3 firstColor = vec3(0, 1, 0); // green
    vec3 middleColor = vec3(0, 0, 1); // blue
    vec3 endColor = vec3(1, 0, 0); // red

    float h = 0.5; // adjust position of middleColor
    vec3 col = mix(mix(firstColor, middleColor, x/h), mix(middleColor, endColor, (x - h)/(1.0 - h)), step(h, x));
    return col;
}


// Render
struct Ray{
    vec3 pos;
    vec3 dir;
    vec3 odir; 
};

ivec3 applyRot(Rot rot, ivec3 v) {
    return ivec3(rot.mat * vec4(v, 1.0));    
}

bool checkHit(in Ray ray, in vec3 nodePos, in uint size, out float tMin, out float tMax) {
    vec3 minSize = nodePos;
    vec3 maxSize = nodePos + vec3(size);

    vec3 isPositive = vec3(ray.odir.x >= 0, ray.odir.y >= 0, ray.odir.z >= 0); 
    vec3 isNegative = 1.0f - isPositive;

    vec3 leftSide  = isPositive * minSize + isNegative * maxSize;
    vec3 rightSide = isPositive * maxSize + isNegative * minSize;

    vec3 leftSideTimesOneOverDir  = (leftSide  - ray.pos) * ray.odir;
    vec3 rightSideTimesOneOverDir = (rightSide - ray.pos) * ray.odir;

    tMin = max(leftSideTimesOneOverDir.x, max(leftSideTimesOneOverDir.y, leftSideTimesOneOverDir.z));
    tMax = min(rightSideTimesOneOverDir.x, min(rightSideTimesOneOverDir.y, rightSideTimesOneOverDir.z));

    // vec3 directionSign = sign(odir);
    // sideMin = vec3(leftSideTimesOneOverDir.x == tMin, leftSideTimesOneOverDir.y == tMin, leftSideTimesOneOverDir.z == tMin) * directionSign;
    // sideMax = vec3(rightSideTimesOneOverDir.x == tMax, rightSideTimesOneOverDir.y == tMax, rightSideTimesOneOverDir.z == tMax) * directionSign;

    return tMax > tMin;
}

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

    uint mat = mats.mats[voxel];
    return GET_MAT(mat);
}

vec4 raycaster(in Ray ray){
    float tMin, tMax = 0;
    float rayLen = 0;
    ivec3 node_size_half = ivec3(NODE_SIZE / 2);
    uint chunk_voxel_size = CHUNK_SIZE * NODE_SIZE;

    ivec3 cellPos;
    ivec3 nodePos;
    uint nodeID;
    Rot rot;
    uint nodeIndex;
    Node node;
    ivec3 voxelPos;
    uint voxelIndex;
    uint voxel;

    if (!checkHit(ray, vec3(0), chunk_voxel_size, tMin, tMax)) {
        return vec4(0);
    }
    ray.pos = ray.pos + ray.dir * (tMin + RAY_POS_OFFSET);  
    rayLen += tMin;   
    
    int counter = 1;
    while (counter < MAX_STEPS) {
        cellPos = ivec3(ray.pos) - (ivec3(ray.pos.x < 0, ray.pos.y < 0, ray.pos.z < 0));

        nodePos = cellPos / NODE_SIZE;
        nodeID = GET_NODE_ID(TO_NODE_ID_INDEX(nodePos));
        rot = GET_ROT_FROM_NODE_ID(nodeID);
        nodeIndex = GET_NODE_INDEX_FROM_NODE_ID(nodeID);
        node = GET_NODE(nodeIndex);

        voxelPos = applyRot(rot, (cellPos % NODE_SIZE) - node_size_half) + node_size_half - rot.offset;

        voxelIndex = TO_VOXEL_INDEX(voxelPos);
        voxel = GET_VOXEL(node, voxelIndex);

        if (voxel != 0) {
            if (DEBUG_STEPS) {
                return vec4(getColorGradient(float(counter) / MAX_STEPS), 1.0);
            }

            return voxelColor(ray, voxel);
        }

        checkHit(ray, vec3(cellPos), 1, tMin, tMax);
        ray.pos = ray.pos + ray.dir * (tMax + RAY_POS_OFFSET);              
        rayLen += tMax;

        if (ray.pos.x < 0 || ray.pos.x >= chunk_voxel_size || ray.pos.y < 0 || ray.pos.y >= chunk_voxel_size || ray.pos.z < 0 || ray.pos.z >= chunk_voxel_size){
            if (DEBUG_STEPS) {
                return vec4(getColorGradient(float(counter) / MAX_STEPS), 1.0);
            }

            return vec4(0);
        }

        counter++;
    }

    return vec4(getColorGradient(1.0), 1.0);
}

void main() {
    vec2 uv = -((2 * gl_FragCoord.xy - renderbuffer.size.xy) / renderbuffer.size.x);

    vec3 ro = POSITION;
    vec3 fwd = DIRECTION;
    vec3 up = vec3(0.,0.,1.);
    vec3 right = normalize(cross(up, fwd));
    up = cross(fwd, right);
    vec3 rd = right * uv.x + up * uv.y + fwd;
    rd = normalize(rd);

    Ray ray = Ray(ro, rd, vec3(1) / rd);
    float tMin, tMax;
    vec4 color = raycaster(ray);

    if (SHIP_TYPE == 1) {
        color.w *= 0.5;
    }

    finalColor = color;

    if (color.w == 0) {
        gl_FragDepth = 1.0;
    } else {
        gl_FragDepth = gl_FragCoord.z;
    }

}
