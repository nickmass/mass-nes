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

uniform vec2 input_size;
uniform vec2 output_size;
uniform sampler2D nes_screen;

vec4 sharp_bilinear(sampler2D tex, vec2 uv)
{
    vec2 texel = uv * input_size;
    vec2 scale = max(floor(output_size / input_size), vec2(1.0, 1.0));

    vec2 texel_floored = floor(texel);
    vec2 s = fract(texel);
    vec2 region_range = 0.5 - 0.5 / scale;

    vec2 center_dist = s - 0.5;
    vec2 f = (center_dist - clamp(center_dist, -region_range, region_range)) * scale + 0.5;

    vec2 mod_texel = texel_floored + f;

    return texture(tex, mod_texel / input_size);
}

void main()
{
    color = vec4(sharp_bilinear(nes_screen, v_tex_coords.xy).zyx, 1.0);
}
