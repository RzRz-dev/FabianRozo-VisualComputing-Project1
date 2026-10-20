#version 300 es
precision highp float;
in vec2 vTexCoord;
// The scene's depth attachment: window-space depth in [0, 1], in .r.
uniform sampler2D uDepth;
uniform bool uLinearize;
out vec4 outColor;
// Must match the scene camera's planes. The near plane sits deliberately
// far out — shrink it toward zero and the raw map collapses to black.
const float n = 100.0;
const float f = 1000.0;
void main() {
  float d = texture(uDepth, vTexCoord).r;      // hyperbolic window depth
  float g = d;
  if (uLinearize) {
    // Undo the projection's depth hyperbola: window → NDC → eye,
    // then normalize eye depth by the far plane. The mapping inverted:
    // https://vcnotebook.gitlab.io/docs/foundations/projection/#mapping-depth
    float zNdc = 2.0 * d - 1.0;
    float zEye = (2.0 * n * f) / (f + n - zNdc * (f - n));
    g = zEye / f;
  }
  outColor = vec4(vec3(1.0 - g), 1.0);         // bright = near, black = deep
}