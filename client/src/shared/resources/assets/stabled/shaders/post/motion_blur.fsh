#version 330
//#if MC >= 26.3
#extension GL_ARB_separate_shader_objects : require
//#endif

// Stabled motion blur (Minecraft 26.2 and newer): blends the freshly rendered frame with the accumulated history.
// BlendConfig.x is the history weight for this frame, already corrected for frame time on the CPU.
// Older versions blend with the GL constant blend colour instead and do not use this shader.

uniform sampler2D InSampler;
uniform sampler2D PrevSampler;

layout(std140) uniform MotionBlurConfig {
    vec4 BlendConfig;
};

//#if MC >= 26.3
layout(location = 0) in vec2 texCoord;

layout(location = 0) out vec4 fragColor;
//#else
in vec2 texCoord;

out vec4 fragColor;
//#endif

void main() {
    vec3 current = texture(InSampler, texCoord).rgb;
    vec3 previous = texture(PrevSampler, texCoord).rgb;
    fragColor = vec4(mix(current, previous, BlendConfig.x), 1.0);
}
