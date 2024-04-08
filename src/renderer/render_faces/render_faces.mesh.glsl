#version 460
#extension GL_EXT_mesh_shader : require

//////////////////////////////////////////////////
// MESH CONFIG

layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;
layout(triangles, max_vertices = 4 * 6, max_primitives = 2 * 6) out;

//////////////////////////////////////////////////
// UNIFORMS

struct VoxelFace {
  vec4 uv;
  uint texture_index;
  bool cullface;
};

struct Voxel {
  vec3 from;
  vec3 to;
  VoxelFace faces[6];
};

layout(std430, set = 1, binding = 0) buffer VoxelBuffer { Voxel voxels[]; };

layout(push_constant) uniform PushConstants {
  mat4 current_view_proj;
  mat4 previous_view_proj;
  vec2 jitter;
}
pc;

struct Task {
  vec3 block_translation;
  uint voxel_offset;
  uint connected_bits;
};
taskPayloadSharedEXT Task task;

//////////////////////////////////////////////////
// OUTPUTS

layout(location = 0) out VertexOut {
  vec4 current_position;
  vec4 previous_position;
  vec3 normal;
  vec2 tex_coords;
  flat uint texture_index;
}
v_out[];

//////////////////////////////////////////////////
// Define vertices for each face of the cube with corrected CCW orientation
const vec3 cube_vertices[6][4] = {
    // Bottom face (adjusted)
    {vec3(0, 0, 0), vec3(0, 1, 0), vec3(1, 1, 0), vec3(1, 0, 0)},
    // Top face
    {vec3(0, 0, 1), vec3(1, 0, 1), vec3(1, 1, 1), vec3(0, 1, 1)},
    // Front face
    {vec3(0, 0, 0), vec3(1, 0, 0), vec3(1, 0, 1), vec3(0, 0, 1)},
    // Back face (adjusted)
    {vec3(0, 1, 0), vec3(0, 1, 1), vec3(1, 1, 1), vec3(1, 1, 0)},
    // Left face (adjusted)
    {vec3(0, 0, 0), vec3(0, 0, 1), vec3(0, 1, 1), vec3(0, 1, 0)},
    // Right face
    {vec3(1, 0, 0), vec3(1, 1, 0), vec3(1, 1, 1), vec3(1, 0, 1)},
};

// Indices and normals remain unchanged
const uvec3 cube_indices[2] = {
    uvec3(0, 1, 3),  // Indices for the first triangle of each face
    uvec3(1, 2, 3),  // Indices for the second triangle of each face
};

const vec3 cube_normals[6] = {
    vec3(0, 0, -1),  // Bottom face
    vec3(0, 0, 1),   // Top face
    vec3(0, -1, 0),  // Front face
    vec3(0, 1, 0),   // Back face
    vec3(-1, 0, 0),  // Left face
    vec3(1, 0, 0),   // Right face
};

struct Face {
  vec3 vertices[4];
  vec3 normal;
  vec2 tex_coords[4];
};

// Function to generate all faces of a voxel
uint generateVoxelFaces(Voxel voxel, out Face faces[6]) {
  uint faceCount = 0;

  for (int i = 0; i < 6; ++i) {
    // Skip faces that are connected to other voxels and are culled
    if ((task.connected_bits & (1 << i)) != 0 && voxel.faces[i].cullface) {
      continue;
    }
    for (int j = 0; j < 4; ++j) {
      faces[faceCount].vertices[j] =
          voxel.from + cube_vertices[i][j] * (voxel.to - voxel.from);
    }
    faces[faceCount].normal = cube_normals[i];
    // TODO: calculate tex_coords
    faces[faceCount].tex_coords[0] = vec2(0.0, 0.0);
    faces[faceCount].tex_coords[1] = vec2(1.0, 0.0);
    faces[faceCount].tex_coords[2] = vec2(1.0, 1.0);
    faces[faceCount].tex_coords[3] = vec2(0.0, 1.0);

    faceCount++;
  }
  return faceCount;
}

void main() {
  uint voxel_index = task.voxel_offset + gl_WorkGroupID.x;

  Voxel voxel = voxels[voxel_index];

  Face faces[6];
  uint faceCount = generateVoxelFaces(voxel, faces);

  SetMeshOutputsEXT(faceCount * 4, faceCount * 2);

  mat4 jitterTransform = mat4(1.0);
  jitterTransform[3] = vec4(pc.jitter, 0.0, 1.0);

  for (int i = 0; i < faceCount; ++i) {
    gl_PrimitiveTriangleIndicesEXT[i * 2] = cube_indices[0] + i * 4;
    gl_PrimitiveTriangleIndicesEXT[i * 2 + 1] = cube_indices[1] + i * 4;

    for (int j = 0; j < 4; ++j) {
      vec4 vertex = vec4(faces[i].vertices[j] + task.block_translation, 1.0);
      vec4 currentPosition = pc.current_view_proj * vertex;
      gl_MeshVerticesEXT[i * 4 + j].gl_Position =
          jitterTransform * currentPosition;
      v_out[i * 4 + j].current_position = currentPosition;
      v_out[i * 4 + j].previous_position = pc.previous_view_proj * vertex;
      v_out[i * 4 + j].normal = faces[i].normal;
      v_out[i * 4 + j].tex_coords = faces[i].tex_coords[j];
      v_out[i * 4 + j].texture_index = voxel.faces[i].texture_index;
    }
  }
}
