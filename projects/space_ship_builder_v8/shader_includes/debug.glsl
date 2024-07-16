#ifndef _DEBUG_GLSL_
#define _DEBUG_GLSL_

#define GET_GRADIENT(i, max) float(i) / max

vec3 get_debug_color_gradient_from_float(float x){
    if (x == 0){
        return vec3(0);
    }

    vec3 firstColor = vec3(0, 1, 0); // green
    vec3 middleColor = vec3(0, 0, 1); // blue
    vec3 endColor = vec3(1, 0, 0); // red

    float h = 0.5; // adjust position of middleColor
    vec3 col = mix(mix(firstColor, middleColor, x/h), mix(middleColor, endColor, (x - h)/(1.0 - h)), step(h, x));
    return col;
}


#endif // _DEBUG_GLSL_
