#version 460

#extension GL_EXT_mesh_shader : require

const uint CHUNK_SIZE = 4;

layout(local_size_x = CHUNK_SIZE, local_size_y = CHUNK_SIZE,
       local_size_z = CHUNK_SIZE) in;

//////////////////////////////////////////////////
// UNIFORMS

struct Block {
  uvec3 position;
  uint voxel_offset;
  uint voxel_len;
  uint connected_bits;  // 6 bits
};

struct Chunk {
  uint len;
  Block blocks[CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE];
};

layout(std430, set = 0, binding = 0) buffer ChunkBuffer { Chunk chunks[]; };

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

layout(std430, set = 0, binding = 1) buffer VoxelBuffer { Voxel voxels[]; };

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

  // Render a single block which may contains multiple voxels
  EmitMeshTasksEXT(voxel_count_lod(block.voxel_len), 1, 1);
}
