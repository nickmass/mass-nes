#pragma stage vertex

in vec2 position;
in vec2 tex_coords;

out vec2 v_tex_coords;

void main() {
    v_tex_coords = tex_coords;
    gl_Position = vec4(position * vec2(1.0, -1.0), 0.0, 1.0);
}

#pragma stage fragment

in vec2 v_tex_coords;
out vec4 color;
uniform sampler2D tex;

void main()
{
    color = vec4(texture(tex, v_tex_coords.xy).xyz, 1.0);
}
