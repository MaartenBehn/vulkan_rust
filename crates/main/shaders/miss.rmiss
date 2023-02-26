#version 460
#extension GL_EXT_ray_tracing : enable

// ------ Payload ------ 
layout(location = 0) rayPayloadEXT Payload {
	vec3 directLight;
	vec3 nextRayOrigin;
	vec3 nextRayDirection;
	vec3 nextFactor;
	bool shadowRayMiss;
	int level;
} payload;
void main() {
	// set color to black
	payload.directLight = vec3(0.0, 0.0, 0.0);
	// shadow ray has not hit an object
	payload.shadowRayMiss = true;
	// no more reflections
	payload.nextRayOrigin = vec3(0.0, 0.0, 0.0);
	payload.nextRayDirection = vec3(0.0, 0.0, 0.0);
}
