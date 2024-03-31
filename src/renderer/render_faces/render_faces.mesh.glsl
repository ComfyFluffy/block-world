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
  uint texture_id;
  bool cullface;
};

struct Voxel {
  vec3 from;
  vec3 to;
  VoxelFace faces[6];
};

layout(std430, set = 1, binding = 0) buffer VoxelBuffer { Voxel voxels[]; };

layout(push_constant) uniform PushConstants {
  mat4 view;
  mat4 proj;
  vec3 camera_pos;
}
pc;

struct Task {
  mat4 model;  // Transformation for the current block
  uint voxel_offset;
  uint connected_bits;
};

taskPayloadSharedEXT Task task;

//////////////////////////////////////////////////
// OUTPUTS

layout(location = 0) out VertexOut {
  vec3 position;
  vec3 normal;
  vec2 tex_coords;
  flat uint texture_index;
}
v_out[];

//////////////////////////////////////////////////
struct FaceVertex {
  vec3 position;
  vec3 normal;
  uint textureIndex;
};

struct Face {
  FaceVertex vertices[4];  // Each face has 4 vertices
};

// Function to generate a single face of a voxel and store it in the provided
// face data structure
void generateFace(vec3 base, vec3 dir1, vec3 dir2, VoxelFace face,
                  out Face outFace) {
  for (int i = 0; i < 4; ++i) {
    vec3 position = base;
    if (i == 1 || i == 2) position += dir1;
    if (i >= 2) position += dir2;

    // Set vertex properties
    outFace.vertices[i].position = position;
    outFace.vertices[i].normal = normalize(
        cross(dir1, dir2));  // Assuming consistent normals for simplicity
    outFace.vertices[i].textureIndex = face.texture_id;
  }
}

// Function to generate all faces of a voxel and emit them
uint generateVoxelFaces(Voxel voxel, out Face[6] faces) {
  uint faceCount = 0;

  // Directions and vectors defining face orientation
  vec3 directions[6] = {vec3(0, 1, 0), vec3(0, -1, 0), vec3(0, 0, -1),
                        vec3(0, 0, 1), vec3(1, 0, 0),  vec3(-1, 0, 0)};
  vec3 right[6] = {vec3(1, 0, 0),  vec3(1, 0, 0),  vec3(1, 0, 0),
                   vec3(-1, 0, 0), vec3(0, 0, -1), vec3(0, 0, 1)};
  vec3 up[6] = {vec3(0, 0, -1), vec3(0, 0, 1), vec3(0, 1, 0),
                vec3(0, 1, 0),  vec3(0, 1, 0), vec3(0, 1, 0)};

  for (int i = 0; i < 6; ++i) {
    if ((task.connected_bits & (1 << i)) != 0 && voxel.faces[i].cullface) {
      continue;
    }
    vec3 base = (i % 2 == 0) ? voxel.from : voxel.to;
    vec3 dir1 = right[i] * (voxel.to - voxel.from);
    vec3 dir2 = up[i] * (voxel.to - voxel.from);
    generateFace(base, dir1, dir2, voxel.faces[i], faces[faceCount++]);
  }
  return faceCount;
}

void main() {
  uint voxel_index = task.voxel_offset + gl_WorkGroupID.x;

  Voxel voxel = voxels[voxel_index];

  Face faces[6];
  uint faceCount = generateVoxelFaces(voxel, faces);

  SetMeshOutputsEXT(4 * faceCount, 2 * faceCount);

  // Emit faces
  for (int i = 0; i < faceCount; ++i) {
    uint baseIndex = i * 4;
    for (int j = 0; j < 4; ++j) {
      gl_MeshVerticesEXT[baseIndex + j].gl_Position =
          pc.proj * pc.view * vec4(faces[i].vertices[j].position, 1.0);
      // Set other vertex attributes as needed, similar to how it's done in
      // `generateFace` TODO
    }

    gl_PrimitiveTriangleIndicesEXT[i * 2] =
        uvec3(baseIndex, baseIndex + 1, baseIndex + 2);
    gl_PrimitiveTriangleIndicesEXT[i * 2 + 1] =
        uvec3(baseIndex, baseIndex + 2, baseIndex + 3);
  }
}
