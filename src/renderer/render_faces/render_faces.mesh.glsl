#version 450
#extension GL_EXT_mesh_shader : require

//////////////////////////////////////////////////
// MESH CONFIG

layout(local_size_x = 1) in;
layout(max_vertices = 4, max_primitives = 2, triangles) out;

//////////////////////////////////////////////////
// UNIFORMS

layout(push_constant) uniform PushConstants {
  mat4 view;
  mat4 proj;
  vec3 camera_pos;
}
pc;

//////////////////////////////////////////////////
// OUTPUTS

layout(location = 0) out VertexOut {
  vec3 position;
  vec3 normal;
  vec2 texCoord;
  flat uint textureIndex;
}
v_out[];

//////////////////////////////////////////////////

struct FaceVertex {
  vec3 position;
  vec3 normal;
  vec2 texCoord;
  uint textureIndex;
};

// Function to calculate the vertices of a cube face based on the direction
void getFaceVertices(uvec3 cubePosition, uint direction,
                     out FaceVertex vertices[4]) {
  vec3 basePos =
      vec3(cubePosition);  // Convert cubePosition to vec3 for arithmetic

  vec2 uvs[4] = {vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0),
                 vec2(0.0, 1.0)};

  // Define vertices based on the direction
  switch (direction) {
    case 0:  // Up
      vertices[0].position = basePos + vec3(0, 1, 0);
      vertices[1].position = basePos + vec3(1, 1, 0);
      vertices[2].position = basePos + vec3(1, 1, 1);
      vertices[3].position = basePos + vec3(0, 1, 1);

      vertices[0].normal = vec3(0, 1, 0);
      break;
    case 1:  // Down
      vertices[0].position = basePos;
      vertices[1].position = basePos + vec3(1, 0, 0);
      vertices[2].position = basePos + vec3(1, 0, 1);
      vertices[3].position = basePos + vec3(0, 0, 1);

      vertices[0].normal = vec3(0, -1, 0);
      break;
    case 2:  // North
      vertices[0].position = basePos;
      vertices[1].position = basePos + vec3(1, 0, 0);
      vertices[2].position = basePos + vec3(1, 1, 0);
      vertices[3].position = basePos + vec3(0, 1, 0);

      vertices[0].normal = vec3(0, 0, -1);
      break;
    case 3:  // South
      vertices[0].position = basePos + vec3(0, 0, 1);
      vertices[1].position = basePos + vec3(1, 0, 1);
      vertices[2].position = basePos + vec3(1, 1, 1);
      vertices[3].position = basePos + vec3(0, 1, 1);

      vertices[0].normal = vec3(0, 0, 1);
      break;
    case 4:  // East
      vertices[0].position = basePos + vec3(1, 0, 0);
      vertices[1].position = basePos + vec3(1, 0, 1);
      vertices[2].position = basePos + vec3(1, 1, 1);
      vertices[3].position = basePos + vec3(1, 1, 0);

      vertices[0].normal = vec3(1, 0, 0);
      break;
    case 5:  // West
      vertices[0].position = basePos + vec3(0, 0, 0);
      vertices[1].position = basePos + vec3(0, 0, 1);
      vertices[2].position = basePos + vec3(0, 1, 1);
      vertices[3].position = basePos + vec3(0, 1, 0);

      vertices[0].normal = vec3(-1, 0, 0);
      break;
    default:
      // Default to Up direction if out of bounds, this should not happen
      vertices[0].position = basePos + vec3(0, 1, 0);
      vertices[1].position = basePos + vec3(1, 1, 0);
      vertices[2].position = basePos + vec3(1, 1, 1);
      vertices[3].position = basePos + vec3(0, 1, 1);

      vertices[0].normal = vec3(0, 1, 0);
      break;
  }
  for (int i = 0; i < 4; ++i) {
    // Assign 0 for texture index for now
    vertices[i].textureIndex = 0;
    vertices[i].texCoord = uvs[i];
  }
}

void main() {
  // Placeholder for data fetching
  uvec3 cubePosition;
  uint direction;
  uint textureIndex;

  // Define vertices for one cube face
  FaceVertex vertices[4];
  getFaceVertices(cubePosition, direction, vertices);

  SetMeshOutputsEXT(4, 2);
  // Emit vertices for the cube face
  for (int i = 0; i < 4; ++i) {
    v_out[i].position = vertices[i].position;
    v_out[i].normal = vertices[i].normal;
    v_out[i].textureIndex = vertices[i].textureIndex;

    gl_MeshVerticesEXT[i].gl_Position = vec4(vertices[i].position, 1.0);
  }

  gl_PrimitiveTriangleIndicesEXT[0] = uvec3(0, 1, 2);
  gl_PrimitiveTriangleIndicesEXT[1] = uvec3(2, 3, 0);
}
