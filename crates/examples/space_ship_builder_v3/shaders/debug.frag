#version 450

// In
layout(location = 0) in vec4 oColor;
// Out
layout(location = 0) out vec4 finalColor;

void main() {
    finalColor = oColor;
}
