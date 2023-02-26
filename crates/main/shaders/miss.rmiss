#version 460
#extension GL_EXT_ray_tracing : enable

layout(location = 0) rayPayloadInEXT HitInfo {
	bool missed;
	vec4 hitValue;
	vec3 hitOrigin;
	vec3 hitNormal;
} hitInfo;

void main() {
	hitInfo.missed = true;
	hitInfo.hitValue = vec4(0.0, 0.0, 0.0, 1.0);
}
