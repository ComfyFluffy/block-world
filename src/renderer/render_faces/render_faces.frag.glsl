#version 450

layout(location = 0) in VertexOut {
  vec3 position;
  vec3 normal;
  vec2 texCoord;
  flat uint textureIndex;
}
v_out;

layout(location = 0) out vec4 fragColor;

void main() {
  // Use the normal directly, but normalize it to be sure it's in the unit range
  vec3 normal = normalize(v_out.normal);

  // Map each component from [-1, 1] to [0, 1]
  vec3 color = normal * 0.5 + 0.5;

  fragColor = vec4(color, 1.0);  // Set alpha to 1.0 for full opacity
}
