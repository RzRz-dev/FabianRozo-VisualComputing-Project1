#version 300 es
precision highp float;
// archetype_blur.frag — one direction of a separable Gaussian, shared by
// the three columns.
// The same shader runs twice per frame: once with uDir = (radius/width, 0)
// to blur horizontally, once with uDir = (0, radius/height) to blur
// vertically. A 2D Gaussian is the outer product of two 1D Gaussians, so
// the two 1D passes compose into the full 2D blur at a fraction of the taps.
in vec2 vTexCoord;
uniform sampler2D uScene;   // the pass input — previous render target
uniform vec2 uDir;          // per-tap UV step: (radius/width, 0)
                            // or (0, radius/height)
out vec4 outColor;
// 9-tap weights (σ ≈ 2): a center tap plus four symmetric pairs.
const float w0 = 0.2270270270;
const float w1 = 0.1945945946;
const float w2 = 0.1216216216;
const float w3 = 0.0540540541;
const float w4 = 0.0162162162;
void main() {
  vec3 c  = texture(uScene, vTexCoord).rgb * w0;
  c += texture(uScene, vTexCoord + uDir * 1.0).rgb * w1;
  c += texture(uScene, vTexCoord - uDir * 1.0).rgb * w1;
  c += texture(uScene, vTexCoord + uDir * 2.0).rgb * w2;
  c += texture(uScene, vTexCoord - uDir * 2.0).rgb * w2;
  c += texture(uScene, vTexCoord + uDir * 3.0).rgb * w3;
  c += texture(uScene, vTexCoord - uDir * 3.0).rgb * w3;
  c += texture(uScene, vTexCoord + uDir * 4.0).rgb * w4;
  c += texture(uScene, vTexCoord - uDir * 4.0).rgb * w4;
  outColor = vec4(c, 1.0);
}