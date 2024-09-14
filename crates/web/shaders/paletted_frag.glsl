#version 300 es

precision mediump float;
precision lowp sampler2D;
precision highp usampler2D;

in vec2 v_tex_coords;
out vec4 color;

uniform usampler2D nes_screen;
uniform sampler2D nes_palette;

void main() {
  uvec4 texel = texture(nes_screen, v_tex_coords);
  int index = int(texel.r) | (int(texel.g) << 8);

  color = texelFetch(nes_palette, ivec2(index % 64, index / 64), 0);
}
