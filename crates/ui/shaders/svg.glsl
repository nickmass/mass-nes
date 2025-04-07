#pragma stage vertex
uniform uint highlight;
uniform vec4 highlight_color;

in uint highlight_mask;
in vec2 position;
in vec4 color;

out vec4 v_color;

void main() {
    v_color = color;
    if ((highlight & highlight_mask) != uint(0)) {
        v_color = highlight_color;
    }
    vec2 out_position = position * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
    gl_Position = vec4(out_position, 0.0, 1.0);
}

#pragma stage fragment

in vec4 v_color;
out vec4 color;

void main() {
    color = v_color;
}
