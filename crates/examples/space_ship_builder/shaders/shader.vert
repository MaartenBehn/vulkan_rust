#version 450

layout(location = 0) in vec3 vPosition;
layout(location = 1) in vec4 vColor;
layout(location = 2) in vec3 vUV;

layout(location = 0) out vec3 oPos;
layout(location = 1) out vec4 oColor;

layout(binding = 0) uniform RenderBuffer {
  mat4 proj_mat;
  mat4 view_mat;
  vec3 dir;
} ubo;

void main() {
    vec3 vPos = vec3(vPosition.x, vPosition.y, vPosition.z);

    gl_Position = ubo.proj_mat * ubo.view_mat * vec4(vPos, 1.0);
    oPos = vUV;
    oColor = vColor;
}
