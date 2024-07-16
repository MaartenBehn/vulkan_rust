#ifndef _RAY_GLSL_
#define _RAY_GLSL_

struct Ray{
    vec3 pos;
    vec3 dir;
    vec3 odir; // = 1 / dir
};

Ray init_ray(vec3 pos, vec3 dir, vec2 coord, vec2 res){
    vec2 uv = ((coord * 2 - res) / res.y) * vec2(-1);

    vec3 ro = pos;
    vec3 fwd = dir;
    vec3 up = vec3(0.,0.,1.);
    vec3 right = normalize(cross(up,fwd));
    up = cross(fwd,right);
    vec3 rd = right * uv.x + up * uv.y + fwd;
    rd = normalize(rd);

    return Ray(ro, rd, vec3(1) / rd);
}

bool aabb_ray_test(in Ray ray, in vec3 minPos, in vec3 maxPos, out float tMin, out float tMax)
{
    vec3 isPositive = vec3(ray.odir.x > 0, ray.odir.y > 0, ray.odir.z >= 0); // ray.odir = 1.0 / ray.dir
    vec3 isNegative = 1.0f - isPositive;

    vec3 leftSide  = isPositive * minPos + isNegative * maxPos;
    vec3 rightSide = isPositive * maxPos + isNegative * minPos;

    vec3 leftSideTimesOneOverDir  = (leftSide  - ray.pos) * ray.odir;
    vec3 rightSideTimesOneOverDir = (rightSide - ray.pos) * ray.odir;

    tMin = max(leftSideTimesOneOverDir.x, max(leftSideTimesOneOverDir.y, leftSideTimesOneOverDir.z));
    tMax = min(rightSideTimesOneOverDir.x, min(rightSideTimesOneOverDir.y, rightSideTimesOneOverDir.z));

    // vec3 directionSign = sign(odir);
    // sideMin = vec3(leftSideTimesOneOverDir.x == tMin, leftSideTimesOneOverDir.y == tMin, leftSideTimesOneOverDir.z == tMin) * directionSign;
    // sideMax = vec3(rightSideTimesOneOverDir.x == tMax, rightSideTimesOneOverDir.y == tMax, rightSideTimesOneOverDir.z == tMax) * directionSign;

    return tMax > tMin;
}

#endif // _RAY_GLSL_
