#version 460

#extension GL_EXT_mesh_shader : require

layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

//////////////////////////////////////////////////
// UNIFORMS

struct Block {
  uint voxel_offset;
  uint voxel_len;
  uint connected_bits;  // 6 bits, can be u8
};

const uint CHUNK_SIZE = 16;
struct Chunk {
  Block blocks[CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
};

layout(std430, set = 0, binding = 0) buffer ChunkBuffer { Chunk chunks[]; };
layout(std430, set = 0, binding = 1) buffer IndexBuffer { uvec2 indices[]; };

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

//////////////////////////////////////////////////
// OUTPUTS

struct Task {
  vec3 block_translation;
  uint voxel_offset;
  uint connected_bits;
};
taskPayloadSharedEXT Task task;

// Used to implement LOD (TODO)
uint voxel_count_lod(uint voxels_for_current_block) {
  const uint LOD_LEVELS[] = {1, 4, 8, 16, 32,
                             /*full*/};
  return voxels_for_current_block;
}

void main() {
  uvec2 index = indices[gl_GlobalInvocationID.x];
  uint chunk_index = index.x;
  uint block_index = index.y;
  Block block = chunks[chunk_index].blocks[block_index];

  task.voxel_offset = block.voxel_offset;
  task.connected_bits = block.connected_bits;
  task.block_translation =  // x, y, z
      vec3(block_index % CHUNK_SIZE, (block_index / CHUNK_SIZE) % CHUNK_SIZE,
           block_index / (CHUNK_SIZE * CHUNK_SIZE));

  if (block.voxel_len == 0) {
    return;
  }
  // Render a single block which may contains multiple voxels
  EmitMeshTasksEXT(voxel_count_lod(block.voxel_len), 1, 1);
}
