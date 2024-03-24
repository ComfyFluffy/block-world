#version 450
#extension GL_EXT_mesh_shader : enable
layout(local_size_x = X, local_size_y = Y,
       local_size_z = Z) in; // Typical limit: 128 invocations.
layout(triangles) out;       // May also be points or lines.
layout(max_vertices = V,
       max_primitives = P) out;       // Typical limit: 256 vert/prim.
layout(location = 0) out vec4 out0[]; // Per-vertex.
layout(location = 1) perprimitiveEXT out vec4 out1[]; // Per-primitive.
void main() {
  // Typical compute built-ins: gl_NumWorkGroups, gl_WorkGroupID,
  // gl_LocalInvocationID, etc. Typical subgroup functionality: gl_NumSubgroups,
  // gl_SubgroupID, subgroupElect(), etc.
  SetMeshOutputsEXT(ACTUAL_V, ACTUAL_P);
  gl_MeshVerticesEXT[FOO].gl_Position = vec4(…);
  gl_PrimitiveTriangleIndicesEXT[BAR] = uvec3(…);
}
