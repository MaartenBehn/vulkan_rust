#ifndef _DEBUG_GLSL_
#define _DEBUG_GLSL_

vec4 step_count_color(uint step_count, uint max_steps) {
    return vec4(1, 1, 1, 0) * (float(step_count) / float(max_steps));
}

#endif // _DEBUG_GLSL_
