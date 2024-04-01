#version 460

layout(location = 0) in VertexOut {
  vec3 position;
  vec3 normal;
  vec2 tex_coords;
  flat uint texture_index;
}
v_out;

layout(location = 0) out vec4 fragColor;

void main() {
  // Map each component from [-1, 1] to [0, 1]
  vec3 color = v_out.normal * 0.5 + 0.5;

  fragColor = vec4(color, 1.0);  // Set alpha to 1.0 for full opacity
}
