
in vec2 v_tex_coords;
out vec4 color;

uniform usampler2D nes_screen;
uniform sampler2D nes_palette;

vec4 sample_screen(vec2 uv) {
    uvec4 texel = texture(nes_screen, uv);
    int index = int(texel.r) | (int(texel.g) << 8);

    return texelFetch(nes_palette, ivec2(index % 64, index / 64), 0);
}

void main() {
    color = sample_screen(v_tex_coords);
}
