#ifndef _NODE_ID_GLSL_
#define _NODE_ID_GLSL_

#define NODE_SIZE 4
#define HALF_NODE_SIZE 2

struct Rot {
    mat4 mat;
    ivec3 offset;
};
Rot GET_ROT_FROM_NODE_ID(uint nodeID) {
    uint index_nz1 = nodeID & 3;
    uint index_nz2 = (nodeID >> 2) & 3;
    uint index_nz3 = 3 - index_nz1 - index_nz2;

    int row_1_sign = (nodeID & (1 << 4)) == 0 ? 1 : -1;
    int row_2_sign = (nodeID & (1 << 5)) == 0 ? 1 : -1;
    int row_3_sign = (nodeID & (1 << 6)) == 0 ? 1 : -1;

    mat4 mat = mat4(0);
    mat[index_nz1][0] = row_1_sign;
    mat[index_nz2][1] = row_2_sign;
    mat[index_nz3][2] = row_3_sign;
    mat[3][3] = 1;

    Rot rot = Rot(mat, ivec3(row_1_sign == -1, row_2_sign == -1, row_3_sign == -1));
    return rot;
}
#define GET_NODE_INDEX_FROM_NODE_ID(nodeID) nodeID >> 7

struct Node {
    uint voxels[(NODE_SIZE * NODE_SIZE * NODE_SIZE) / 4];
};
#define GET_VOXEL_INDEX_FROM_VOXEL_POS(pos) ((pos.z * NODE_SIZE * NODE_SIZE) + (pos.y * NODE_SIZE) + pos.x)
#define GET_VOXEL(node, index) (node.voxels[index / 4] >> ((index % 4) * 8)) & 255

#define APPLY_ROT(rot, v) ivec3(rot.mat * vec4(v, 1.0))
#define ROTATE_VOXEL_POS(voxel_pos, rot) (APPLY_ROT(rot, (voxel_pos % NODE_SIZE) - HALF_NODE_SIZE) + HALF_NODE_SIZE - rot.offset)

#define GET_MAT_VECTOR_FROM_MAT_INT(mat) (vec4(float(mat & 255) / 255.0, float((mat >> 8) & 255) / 255.0, float((mat >> 16) & 255) / 255.0, float((mat >> 24) & 255) / 255.0))

#endif // _NODE_ID_GLSL_
