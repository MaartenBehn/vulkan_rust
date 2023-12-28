#version 450

layout(location = 0) in vec3 oPos;
layout(location = 1) in vec4 oColor;

layout(location = 0) out vec4 finalColor;

layout(binding = 0) uniform RenderBuffer {
  mat4 proj_mat;
  mat4 view_mat;
  vec3 dir;
} ubo;

#define POSITION oPos
#define DIRECTION ubo.dir;

#define NODE_SIZE 8
#define MAX_STEPS 100
#define RAY_POS_OFFSET 0.0001
#define SCREEN_SIZE vec2(1024, 576)

struct Node {
    uint voxels[(NODE_SIZE * NODE_SIZE * NODE_SIZE) / 4];
};

#define GET_VOXEL(node, index) (node.voxels[index / 4] >> (index % 4)) & 256

layout(binding = 1) buffer Nodes {
    Node nodes[];
} octtreeBuffer;


// Render
struct Ray{
    vec3 pos;
    vec3 dir;
    vec3 odir; 
};

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

vec4 raycaster(in Ray ray){
    
    float tMin, tMax = 0;
    float rayLen = 0;
    uvec3 voxelPos = uvec3(0);

    bool hit = checkHit(ray, vec3(0), 8, tMin, tMax); 
    if (!hit) {
        return vec4(1.0, 0.0, 0.0, 1.0);
    }
    ray.pos = ray.pos + ray.dir * (tMin + RAY_POS_OFFSET);  
    rayLen += tMin;   
    
    int counter = 0;
    while (counter < MAX_STEPS) {
        voxelPos = uvec3(ray.pos) - uvec3(ray.pos.x < 0, ray.pos.y < 0, ray.pos.z < 0);

        if (voxelPos.x % 2 == 0 && voxelPos.y % 2 == 0 && voxelPos.z % 2 == 0) {
            return vec4(0.5, 0.5, 0.5, 1.0);
        }

        checkHit(ray, vec3(voxelPos), 1, tMin, tMax);                     
        ray.pos = ray.pos + ray.dir * (tMax + RAY_POS_OFFSET);              
        rayLen += tMax; 

        if (ray.pos.x < 0 || ray.pos.x >= 8 || ray.pos.y < 0 || ray.pos.y >= 8 || ray.pos.z < 0 || ray.pos.z >= 8){
            break;
        }

        counter++;
    }

    return vec4(0);
}

void main() {
    vec2 uv = ((gl_FragCoord.xy * 2 - SCREEN_SIZE) / SCREEN_SIZE.y) * vec2(-0.5);

    vec3 ro = POSITION;
    vec3 fwd = DIRECTION;
    vec3 up = vec3(0.,1.,0.);
    vec3 right = normalize(cross(up, fwd));
    up = cross(fwd,right);
    vec3 rd = right * uv.x + up * uv.y + fwd;
    rd = normalize(rd);

    Ray ray = Ray(ro, rd, vec3(1) / rd);

    float tMin, tMax;
    
    vec4 color = raycaster(ray);

    finalColor = color;
}
