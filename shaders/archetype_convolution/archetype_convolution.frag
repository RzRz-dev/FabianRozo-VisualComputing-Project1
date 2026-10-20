#version 300 es
precision highp float;

out vec4 outColor;
in vec2 vTexCoord;

uniform sampler2D uSource;
uniform vec2  uTexelSize;   // 1/texture width, 1/texture height
uniform int   uKernel;      // see the table below
uniform float uBias;
uniform bool  uOriginal;

// 0  Ridge / Sobel X
// 1  Emboss
// 2  Sharpen
// 3  Box blur
// 4  Identity
// 5  Gaussian blur
// 6  Sobel Y
// 7  Laplacian
// 8  Outline
// 9  Sharpen strong
// 10 Motion blur (diagonal)
// 11 Sobel magnitude (uses kernels 0 and 6 together)

const float kRidge[9]       = float[9](-1.,0.,1., -2.,0.,2., -1.,0.,1.);
const float kEmboss[9]      = float[9](-2.,-1.,0., -1.,0.,1., 0.,1.,2.);
const float kSharpen[9]     = float[9](0.,-1.,0., -1.,5.,-1., 0.,-1.,0.);
const float kBlur[9]        = float[9](1./9.,1./9.,1./9., 1./9.,1./9.,1./9., 1./9.,1./9.,1./9.);
const float kIdentity[9]    = float[9](0.,0.,0., 0.,1.,0., 0.,0.,0.);
const float kGaussian[9]    = float[9](1./16.,2./16.,1./16., 2./16.,4./16.,2./16., 1./16.,2./16.,1./16.);
const float kSobelY[9]      = float[9](-1.,-2.,-1., 0.,0.,0., 1.,2.,1.);
const float kLaplacian[9]   = float[9](0.,1.,0., 1.,-4.,1., 0.,1.,0.);
const float kOutline[9]     = float[9](-1.,-1.,-1., -1.,8.,-1., -1.,-1.,-1.);
const float kSharpStrong[9] = float[9](-1.,-1.,-1., -1.,9.,-1., -1.,-1.,-1.);
const float kMotion[9]      = float[9](1./3.,0.,0., 0.,1./3.,0., 0.,0.,1./3.);

float weightFor(int k, int i) {
  if (k == 0)  return kRidge[i];
  if (k == 1)  return kEmboss[i];
  if (k == 2)  return kSharpen[i];
  if (k == 3)  return kBlur[i];
  if (k == 4)  return kIdentity[i];
  if (k == 5)  return kGaussian[i];
  if (k == 6)  return kSobelY[i];
  if (k == 7)  return kLaplacian[i];
  if (k == 8)  return kOutline[i];
  if (k == 9)  return kSharpStrong[i];
  if (k == 10) return kMotion[i];
  return kIdentity[i];
}

void main() {
  if (uOriginal) {
    outColor = texture(uSource, vTexCoord);
    return;
  }

  // Sobel magnitude: sqrt(gx^2 + gy^2), a combination of two kernels
  if (uKernel == 11) {
    vec3 gx = vec3(0.0);
    vec3 gy = vec3(0.0);
    for (int i = 0; i < 9; i++) {
      vec2 offset = vec2(float(i % 3) - 1.0, float(i / 3) - 1.0) * uTexelSize;
      vec3 s = texture(uSource, vTexCoord + offset).rgb;
      gx += s * kRidge[i];
      gy += s * kSobelY[i];
    }
    outColor = vec4(clamp(sqrt(gx * gx + gy * gy) + uBias, 0.0, 1.0), 1.0);
    return;
  }

  vec3 c = vec3(0.0);
  for (int i = 0; i < 9; i++) {
    vec2 offset = vec2(float(i % 3) - 1.0, float(i / 3) - 1.0) * uTexelSize;
    c += texture(uSource, vTexCoord + offset).rgb * weightFor(uKernel, i);
  }

  outColor = vec4(clamp(c + uBias, 0.0, 1.0), 1.0);
}