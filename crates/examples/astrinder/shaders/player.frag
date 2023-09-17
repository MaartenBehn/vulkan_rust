#version 450

layout(binding = 0) uniform RenderUniformBufferObject {
    vec4 cam;  // Pos X, Pos Y, Rot, Scale
    vec4 data; // Aspect
} render_ubo;

layout(location = 0) out vec4 finalColor;

void main() {
    finalColor = vec4(22.0 / 255.0, 184.0 / 255.0, 243.0 / 255.0, 1.0);
}
