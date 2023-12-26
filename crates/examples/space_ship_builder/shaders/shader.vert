#version 450

layout(location = 0) in vec3 vPosition;
layout(location = 1) in vec4 vColor;

layout(location = 0) out vec4 oColor;

layout(binding = 0) uniform RenderBuffer {
  mat4 projectionViewMatrix;
} ubo;

void main() {
    oColor = vColor;

    vec3 vPos = vec3(vPosition.x, vPosition.y, vPosition.z);

    gl_Position = ubo.projectionViewMatrix * vec4(vPos, 1.0);
}
