#version 460

layout(location = 0) in VertexOut {
  vec4 current_position;
  vec4 previous_position;
  vec3 normal;
  vec2 tex_coords;
  flat uint texture_index;
}
v_out;

layout(location = 0) out vec4 frag_color;
layout(location = 1) out vec2 motion_vector;

void main() {
  motion_vector = v_out.previous_position.xy / v_out.previous_position.w -
                  v_out.current_position.xy / v_out.current_position.w;

  // Map each component from [-1, 1] to [0, 1]
  vec3 color = v_out.normal * 0.5 + 0.5;

  frag_color = vec4(color, 1.0);  // Set alpha to 1.0 for full opacity
}
