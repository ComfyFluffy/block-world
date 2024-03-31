#version 460

#extension GL_EXT_mesh_shader : require

layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

//////////////////////////////////////////////////
// UNIFORMS

struct Block {
  uvec3 position;
  uint voxel_offset;
  uint voxel_len;
  uint connected_bits;  // 6 bits
};

layout(std430, set = 0, binding = 0) buffer BlockBuffer { Block blocks[]; };

struct VoxelFace {
  vec4 uv;
  uint texture_id;
  uint cullface;
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
  mat4 model;  // Transformation for the current block
  uint voxel_offset;
  uint connected_bits;
};
taskPayloadSharedEXT Task task;

// Used to implement LOD (TODO)
uint voxel_count_lod(uint voxels_for_current_block) {
  const uint LOD_LEVELS[] = {1, 4, 8, 16, 32, 64,
                             /*full*/};
  return voxels_for_current_block;
}

void main() {
  // use workgroup ID to index ChunkBuffer, and local id to index Chunk::blocks
  uint chunk_index = gl_WorkGroupID.x;
  uint block_index = CHUNK_SIZE * CHUNK_SIZE * gl_LocalInvocationID.z +
                     CHUNK_SIZE * gl_LocalInvocationID.y +
                     gl_LocalInvocationID.x;

  Block block = chunks[chunk_index].blocks[block_index];

  task.voxel_offset = block.voxel_offset;
  task.connected_bits = block.connected_bits;

  if (block.voxel_len == 0) {
    return;
  }
  // Render a single block which may contains multiple voxels
  EmitMeshTasksEXT(voxel_count_lod(block.voxel_len), 1, 1);
}
