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
	vec4 lightDirection;
	vec4 lightColor;
	uint maxDepth;
    uint rays_per_pixel;
    uint render_mode;
} scene;
layout(binding = 3, set = 0) readonly buffer Vertices { Vertex v[]; } vertices;
layout(binding = 4, set = 0) readonly buffer Indices { uint i[]; } indices;
layout(binding = 5, set = 0) readonly buffer GeometryInfos { GeometryInfo g[]; } geometryInfos;
layout(binding = 6, set = 0) uniform sampler2D textures[];


// ------ HitInfo ------ 
layout(location = 0) rayPayloadInEXT HitInfo {
	bool missed;
	vec4 hitValue;
	vec3 hitOrigin;
	vec3 hitNormal;
} hitInfo;
hitAttributeEXT vec2 attribs;


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

    // Origen
    hitInfo.hitOrigin = gl_WorldRayOriginEXT + gl_WorldRayDirectionEXT * gl_HitTEXT;

    // Normal
	const vec3 barycentricCoords = vec3(1.0f - attribs.x - attribs.y, attribs.x, attribs.y);
	vec3 normal = normalize(v0.normal * barycentricCoords.x + v1.normal * barycentricCoords.y + v2.normal * barycentricCoords.z);
    normal = normalize(geometryInfo.transform * vec4(normal, 0.0)).xyz;
    hitInfo.hitNormal = normal;

    //Color
    vec3 vertexColor = v0.color * barycentricCoords.x + v1.color * barycentricCoords.y + v2.color * barycentricCoords.z;
    vec3 baseColor = geometryInfo.baseColor.xyz;
    vec3 color = vertexColor * baseColor;
    hitInfo.hitValue = vec4(color, 1 - geometryInfo.metallicFactor);

    hitInfo.missed = false;
}
