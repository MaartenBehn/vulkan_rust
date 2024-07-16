#ifndef _DDA_GLSL_
#define _DDA_GLSL_

#include "./ray.glsl"
#define USE_BRANCHLESS_DDA true

struct DDA {
    vec3 pos;
    vec3 delta_dist;
    vec3 step;
    vec3 side_dist;
    vec3 mask;
    vec3 upper_bound;
    bool out_of_bounds;
};

DDA init_DDA(in Ray ray, in vec3 start_pos, in ivec3 upper_bound) {
    vec3 cell = floor(start_pos);
    vec3 delta_dist = abs(vec3(length(ray.dir)) / ray.dir);
    vec3 step = sign(ray.dir);
    vec3 side_dist = (step * (cell - start_pos) + (step * 0.5) + 0.5) * delta_dist;
    vec3 mask;

    return DDA(start_pos, delta_dist, step, side_dist, mask, upper_bound, false);
}

DDA step_DDA(in DDA dda) {
    // Implementaion inspirend by: https://www.shadertoy.com/view/4dX3zl
    #ifdef USE_BRANCHLESS_DDA
        dda.mask = vec3(lessThanEqual(dda.side_dist.xyz, min(dda.side_dist.yzx, dda.side_dist.zxy)));
        dda.side_dist += vec3(dda.mask) * dda.delta_dist;
        dda.pos += dda.mask * dda.step;
    #else
        if (dda.side_dist.x < dda.side_dist.y) {
            if (dda.side_dist.x < dda.side_dist.z) {
                dda.side_dist.x += dda.delta_dist.x;
                dda.pos.x += dda.step.x;
                dda.mask = vec3(1, 0, 0);
            }
            else {
                dda.side_dist.z += dda.delta_dist.z;
                dda.pos.z += dda.step.z;
                dda.mask = vec3(0, 0, 1);
            }
        }
        else {
            if (dda.side_dist.y < dda.side_dist.z) {
                dda.side_dist.y += dda.delta_dist.y;
                dda.pos.y += dda.step.y;
                dda.mask = vec3(0, 1, 0);
            }
            else {
                dda.side_dist.z += dda.delta_dist.z;
                dda.pos.z += dda.step.z;
                dda.mask = vec3(0, 0, 1);
            }
        }
    #endif

    dda.out_of_bounds = (dda.mask.x != 0 && (dda.pos.x <= -1 || dda.pos.x >= dda.upper_bound.x)
    || dda.mask.y != 0 && (dda.pos.y <= -1 || dda.pos.y >= dda.upper_bound.y)
    || dda.mask.z != 0 && (dda.pos.z <= -1 || dda.pos.z >= dda.upper_bound.z));

    return dda;
}

float get_DDA_t(in DDA dda) {
    vec3 side_dist = dda.mask * dda.side_dist;
    return side_dist.x + side_dist.y + side_dist.z;
}

#endif // __DDA_GLSL__
