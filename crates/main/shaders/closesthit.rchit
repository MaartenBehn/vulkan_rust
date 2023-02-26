#version 460
#extension GL_EXT_ray_tracing : enable
#extension GL_EXT_nonuniform_qualifier : enable

// ------ Bindings ------ 
struct Vertex {
    vec3 pos;
    vec3 normal;
    vec3 color;
    vec2 uvs;
};

struct GeometryInfo {
    mat4 transform;
    vec4 baseColor;
    int baseColorTextureIndex;
    float metallicFactor;
    uint vertexOffset;
    uint indexOffset;
};

layout(binding = 0, set = 0) uniform accelerationStructureEXT topLevelAS;
layout(binding = 2, set = 0) uniform SceneData {
	mat4 invertedView;
	mat4 invertedProj;
	vec4 lightPos;
	vec4 lightColor;
	uint maxDepth;
    uint rays_per_pixel;
    uint render_mode;
} scene;
layout(binding = 3, set = 0) readonly buffer Vertices { Vertex v[]; } vertices;
layout(binding = 4, set = 0) readonly buffer Indices { uint i[]; } indices;
layout(binding = 5, set = 0) readonly buffer GeometryInfos { GeometryInfo g[]; } geometryInfos;
layout(binding = 6, set = 0) uniform sampler2D textures[];


// ------ Payload ------ 
layout(location = 0) rayPayloadInEXT Payload {
	vec3 directLight;
	vec3 nextRayOrigin;
	vec3 nextRayDirection;
	vec3 nextFactor;
	bool shadowRayMiss;
	int level;
    uint pass;
} payload;
layout(location = 1) rayPayloadEXT bool isShadowed;

hitAttributeEXT vec2 attribs;

const float PI = 3.14159265359;

// Hash Functions for GPU Rendering, Jarzynski et al.
// http://www.jcgt.org/published/0009/03/02/
vec3 random_pcg3d(uvec3 v) {
  v = v * 1664525u + 1013904223u;
  v.x += v.y*v.z; v.y += v.z*v.x; v.z += v.x*v.y;
  v ^= v >> 16u;
  v.x += v.y*v.z; v.y += v.z*v.x; v.z += v.x*v.y;
  return vec3(v) * (1.0/float(0xffffffffu));
}

mat3 getNormalSpace(in vec3 normal) {
   vec3 someVec = vec3(1.0, 0.0, 0.0);
   float dd = dot(someVec, normal);
   vec3 tangent = vec3(0.0, 1.0, 0.0);
   if(1.0 - abs(dd) > 1e-6) {
     tangent = normalize(cross(someVec, normal));
   }
   vec3 bitangent = cross(normal, tangent);
   return mat3(tangent, bitangent, normal);
}

void main() {
    GeometryInfo geometryInfo = geometryInfos.g[gl_GeometryIndexEXT];

    // Fetch vertices
    uint vertexOffset = geometryInfo.vertexOffset;
    uint indexOffset = geometryInfo.indexOffset + (3 * gl_PrimitiveID);

    uint i0 = vertexOffset + indices.i[indexOffset];
    uint i1 = vertexOffset + indices.i[indexOffset + 1];
    uint i2 = vertexOffset + indices.i[indexOffset + 2];

    Vertex v0 = vertices.v[i0];
	Vertex v1 = vertices.v[i1];
	Vertex v2 = vertices.v[i2];

    // interpolate with barycentric coordinate
    const vec3 barys = vec3(1.0f - attribs.x - attribs.y, attribs.x, attribs.y);
    vec3 localNormal = normalize(v0.normal * barys.x + v1.normal * barys.y + v2.normal * barys.z);
    vec3 localPosition = v0.pos * barys.x + v1.pos * barys.y + v2.pos * barys.z;
    vec2 texCoords = v0.uvs * barys.x + v1.uvs * barys.y + v2.uvs * barys.z;

    // transform to world space
    vec3 normal = normalize(geometryInfo.transform * vec4(localNormal, 0.0)).xyz;
    vec3 position = gl_ObjectToWorldEXT * vec4(localPosition, 1.0);

    //Color
    vec3 vertexColor = v0.color * barys.x + v1.color * barys.y + v2.color * barys.z;
    vec3 baseColor = geometryInfo.baseColor.xyz;
    vec3 color = vertexColor * baseColor;

    // Ligth
    vec3 lightDir = normalize(scene.lightPos.xyz - position);
    float lightDist = length(scene.lightPos.xyz - position);
    float lightAttenuation = min(10.0, 1.0 / (lightDist*lightDist));
    float lightIntensity = (scene.lightColor.x + scene.lightColor.y + scene.lightColor.z) / 3;
    // diffuse shading (direct light)
    vec3 radiance = vec3(0.0, 0.0, 0.0); // no ambient term

    // prepare shadow ray
    uint rayFlags = gl_RayFlagsTerminateOnFirstHitEXT | gl_RayFlagsSkipClosestHitShaderEXT;
    float rayMin     = 0.001;
    float rayMax     = length(scene.lightPos.xyz - position);  
    float shadowBias = 0.1;
    uint cullMask = 0xFFu;
    float frontFacing = dot(-gl_WorldRayDirectionEXT, normal);
    vec3 shadowRayOrigin = position + sign(frontFacing) * shadowBias * normal;
    vec3 shadowRayDirection = lightDir;
    payload.shadowRayMiss = false;

    // shot shadow ray
    traceRayEXT(topLevelAS, rayFlags, cullMask, 0u, 0u, 0u, 
        shadowRayOrigin, rayMin, shadowRayDirection, rayMax, 0);
        
    float irradiance = max(dot(lightDir, normal), 0.0) * lightAttenuation * lightIntensity * 5;
    if(irradiance > 0.0) { // if receives light
    radiance += color / PI * irradiance; // diffuse shading
    }
    payload.directLight = radiance;

    // different random value for each pixel and each frame
    vec3 random = random_pcg3d(uvec3(gl_LaunchIDEXT.xy, payload.pass + payload.level));

    // important sampling
    float theta = asin(sqrt(random.y));
    float phi = 2.0 * PI * random.x;
    
    // sampled indirect diffuse direction in normal space
    vec3 localDiffuseDir = vec3(sin(theta) * cos(phi), sin(theta) * sin(phi), cos(theta));
    vec3 diffuseDir = getNormalSpace(normal) * localDiffuseDir;

    payload.nextRayDirection = diffuseDir;

    payload.nextRayOrigin = position;
    payload.nextFactor = color;
}
