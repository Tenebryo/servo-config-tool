#version 450

layout(set=0, binding=0) uniform UniformBlock0 {
  mat4 matrix;
  vec2 viewport;
};

layout(location = 0) in vec3 pos;
layout(location = 1) in vec4 col;

layout(location = 0) out vec4 f_color;
layout(location = 1) out vec2 center;

// Built-in:
// vec4 gl_Position

void main() {
  f_color = col;

  vec4 tpos = matrix * vec4(pos.xyz, 1);

  gl_Position = tpos;

  vec2 vp = viewport;
  center = 0.5 * (tpos.xy + vec2(1)) * vp;
}
