pub const COMPUTE_VERTEX: &str = r#"#version 300 es
precision highp float;
const vec2 POSITIONS[3] = vec2[](
    vec2(-1.0, -1.0),
    vec2(3.0, -1.0),
    vec2(-1.0, 3.0)
);

void main() {
    gl_Position = vec4(POSITIONS[gl_VertexID], 0.0, 1.0);
}
"#;

pub const COMPUTE_FRAGMENT: &str = r#"#version 300 es
precision highp float;

uniform sampler2D u_source;
uniform vec2 u_viewport;
uniform vec2 u_textureSize;
uniform float u_deltaMs;
uniform float u_time;
uniform float u_wind;

out vec4 out_particle;

float wrap(float value, float maxValue) {
    return mod(value + maxValue, maxValue);
}

float hash(vec2 seed) {
    return fract(sin(dot(seed, vec2(12.9898, 78.233))) * 43758.5453123);
}

void main() {
    vec2 texel = gl_FragCoord.xy / u_textureSize;
    vec4 particle = texture(u_source, texel);
    float radius = max(0.5, particle.z);
    float jitter = particle.w;

    float dt = u_deltaMs * 0.001;
    float sway = sin(jitter * 6.2831 + u_time * 0.0013) * (0.5 + radius * 0.08);
    float fall_speed = 24.0 + radius * 18.0;

    vec2 position = particle.xy;
    position.x += (sway + u_wind * 80.0) * dt;
    position.y += fall_speed * dt;

    if (position.y - radius > u_viewport.y + 8.0) {
        vec2 frag = gl_FragCoord.xy;
        float base = hash(frag + vec2(u_time * 0.001, jitter));
        float spread = hash(frag.yx + vec2(jitter, u_time * 0.00037));
        position.x = base * (u_viewport.x + 40.0) - 20.0;
        position.y = -radius - spread * 80.0;
        radius = 0.8 + hash(frag + vec2(base, spread)) * 2.6;
        jitter = hash(vec2(base, spread * 1.618));
    }

    position.x = wrap(position.x, u_viewport.x);

    out_particle = vec4(position, radius, jitter);
}
"#;

pub const RENDER_VERTEX: &str = r#"#version 300 es
precision highp float;

layout (location = 0) in vec2 a_particle_uv;

uniform sampler2D u_particles;
uniform vec2 u_viewport;
uniform float u_pointScale;

out float v_radius;
out float v_alpha;

void main() {
    vec4 particle = texture(u_particles, a_particle_uv);
    vec2 position = particle.xy;
    v_radius = particle.z;
    v_alpha = smoothstep(0.0, 1.0, particle.z / 3.5);

    vec2 clip = vec2(
        (position.x / u_viewport.x) * 2.0 - 1.0,
        1.0 - (position.y / u_viewport.y) * 2.0
    );

    gl_Position = vec4(clip, 0.0, 1.0);
    gl_PointSize = max(1.0, v_radius * u_pointScale);
}
"#;

pub const RENDER_FRAGMENT: &str = r#"#version 300 es
precision highp float;

in float v_radius;
in float v_alpha;

out vec4 fragColor;

void main() {
    vec2 coord = gl_PointCoord * 2.0 - 1.0;
    float dist = dot(coord, coord);
    if (dist > 1.0) {
        discard;
    }
    float softness = smoothstep(1.0, 0.0, dist) * v_alpha;
    fragColor = vec4(vec3(1.0), softness);
}
"#;
