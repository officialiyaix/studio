#version 330
#extension GL_ARB_separate_shader_objects : require

// Stabled motion blur: blends the freshly rendered frame with the accumulated history.
// BlendConfig.x is the history weight for this frame, already corrected for frame time on the CPU.

uniform sampler2D InSampler;
uniform sampler2D PrevSampler;

layout(std140) uniform MotionBlurConfig {
    vec4 BlendConfig;
};

layout(location = 0) in vec2 texCoord;

layout(location = 0) out vec4 fragColor;

void main() {
    vec3 current = texture(InSampler, texCoord).rgb;
    vec3 previous = texture(PrevSampler, texCoord).rgb;
    fragColor = vec4(mix(current, previous, BlendConfig.x), 1.0);
}
