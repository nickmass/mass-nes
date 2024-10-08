
// Adapted from https://www.shadertoy.com/view/WsVSzV

float warp = 0.3; // simulate curvature of CRT monitor
float scan = 0.15; // simulate darkness between scanlines

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
    // squared distance from center
    vec2 uv = v_tex_coords.xy;
    vec2 dc = abs(0.5 - uv);
    dc *= dc;

    // warp the fragment coordinates
    uv.x -= 0.5;
    uv.x *= 1.0 + (dc.y * (0.3 * warp));
    uv.x += 0.5;
    uv.y -= 0.5;
    uv.y *= 1.0 + (dc.x * (0.4 * warp));
    uv.y += 0.5;

    // sample inside boundaries, otherwise set to black
    if (uv.y > 1.0 || uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0) {
        color = vec4(0.0, 0.0, 0.0, 1.0);
    } else {
        // determine if we are drawing in a scanline
        float apply = abs(sin(v_tex_coords.y * input_size.y * 4.0) * 0.5 * scan);
        // sample the texture
        color = vec4(mix(sharp_bilinear(nes_screen, uv).zyx, vec3(0.0), apply), 1.0);
    }
}
