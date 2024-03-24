#version 450

layout(location = 0) in VertexOut {
  vec3 position;
  vec3 normal;
  vec2 texCoord;
  flat uint textureIndex;
}
v_out;
layout(location = 0) out vec4 fragColor;

layout(push_constant) uniform PushConstants {
  mat4 view;
  mat4 proj;
  vec3 camera_pos;
}
pc;
layout(binding = 0) uniform sampler2DArray textures;

void main() {
  vec3 normal = normalize(v_out.normal);
  vec3 light_dir = normalize(vec3(1.0, 1.0, 1.0));
  float light_intensity = max(dot(normal, light_dir), 0.0);

  vec3 view_dir = normalize(pc.camera_pos - v_out.position);
  vec3 reflect_dir = reflect(-light_dir, normal);
  float spec_intensity = pow(max(dot(view_dir, reflect_dir), 0.0), 32);

  vec4 texColor = texture(textures, vec3(v_out.texCoord, v_out.textureIndex));
  fragColor = vec4(texColor.rgb * light_intensity + vec3(0.1) * spec_intensity,
                   texColor.a);
}
