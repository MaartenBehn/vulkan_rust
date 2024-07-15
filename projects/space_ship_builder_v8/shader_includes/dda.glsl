#ifndef _DDA_GLSL_
#define _DDA_GLSL_

#include "./ray.glsl"
#define USE_BRANCHLESS_DDA true

struct DDA {
    ivec3 cell;
    vec3 delta_dist;
    ivec3 step;
    vec3 side_dist;
    bvec3 mask;
    ivec3 upper_bound;
    bool out_of_bounds;
};

DDA init_DDA(in Ray ray, in vec3 start_pos, in ivec3 upper_bound) {
    ivec3 cell = ivec3(start_pos);
    vec3 delta_dist = abs(vec3(length(ray.dir)) / ray.dir);
    ivec3 step = ivec3(sign(ray.dir));
    vec3 side_dist = (step * (vec3(cell) - start_pos) + (step * 0.5) + 0.5) * delta_dist;
    bvec3 mask;

    return DDA(cell, delta_dist, step, side_dist, mask, upper_bound, false);
}

DDA step_DDA(in DDA dda) {
    // Implementaion inspirend by: https://www.shadertoy.com/view/4dX3zl
    #ifdef USE_BRANCHLESS_DDA
        dda.mask = lessThanEqual(dda.side_dist.xyz, min(dda.side_dist.yzx, dda.side_dist.zxy));
        dda.side_dist += vec3(dda.mask) * dda.delta_dist;
        dda.cell += ivec3(dda.mask) * dda.step;
    #else
        if (dda.side_dist.x < dda.side_dist.y) {
            if (dda.side_dist.x < dda.side_dist.z) {
                dda.side_dist.x += dda.delta_dist.x;
                dda.cell.x += dda.step.x;
                dda.mask = bvec3(true, false, false);
            }
            else {
                dda.side_dist.z += dda.delta_dist.z;
                dda.cell.z += dda.step.z;
                dda.mask = bvec3(false, false, true);
            }
        }
        else {
            if (dda.side_dist.y < dda.side_dist.z) {
                dda.side_dist.y += dda.delta_dist.y;
                dda.cell.y += dda.step.y;
                dda.mask = bvec3(false, true, false);
            }
            else {
                dda.side_dist.z += dda.delta_dist.z;
                dda.cell.z += dda.step.z;
                dda.mask = bvec3(false, false, true);
            }
        }
    #endif

    dda.out_of_bounds = (dda.mask.x && (dda.cell.x <= -1 || dda.cell.x >= dda.upper_bound.x)
    || dda.mask.y && (dda.cell.y <= -1 || dda.cell.y >= dda.upper_bound.y)
    || dda.mask.z && (dda.cell.z <= -1 || dda.cell.z >= dda.upper_bound.z));

    return dda;
}

#endif // __DDA_GLSL__
