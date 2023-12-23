#version 450

layout(location = 0) in vec3 vPosition;
layout(location = 1) in vec3 vColor;

layout(location = 0) out vec3 oColor;

layout(binding = 0) uniform RenderBuffer {
  mat4 projectionViewMatrix;
} ubo;

void main() {
    oColor = vColor;

    vec3 vPos = vec3(vPosition.x, vPosition.y, vPosition.z);
    vPos.y *= -1;

    gl_Position = ubo.projectionViewMatrix * vec4(vPos, 1.0);
}
