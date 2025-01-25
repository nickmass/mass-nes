#pragma stage vertex

in vec2 position;
in vec2 tex_coords;

out vec2 v_tex_coords;

void main() {
    v_tex_coords = tex_coords;
    gl_Position = vec4(position, 0.0, 1.0);
}

#pragma stage fragment

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
