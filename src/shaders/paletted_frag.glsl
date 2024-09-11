#version 140

in vec2 v_tex_coords;
out vec4 color;

uniform isampler2D tex;
uniform sampler2D palette;

void main() {
  ivec4 index = texture(tex, v_tex_coords);
  color = texelFetch(palette, ivec2(index.x % 64, index.x / 64), 0);
}
