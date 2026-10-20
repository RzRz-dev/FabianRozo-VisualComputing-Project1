#version 300 es
precision highp float;
// ── Vertex-in (interpolated, eye-space) ──────────────────────────────────
in vec3 vNormal;
in vec3 vEyePos;
// ── Uniforms ─────────────────────────────────────────────────────────────
// Light position in eye space — the host computes view × world_light
// and pushes the result here so the fragment shader can compute light
// direction without needing the view matrix again.
uniform vec3 uLightPosition;
// Material properties.
uniform vec3  uColor;       // base material colour (R, G, B in [0,1])
uniform float uAmbient;     // ambient coefficient (low — e.g., 0.15)
uniform float uSpecPower;   // specular exponent (e.g., 32). Not uShininess:
                            // engines, p5 among them, reserve that name for
                            // their own material state and reassert it on
                            // every draw, clobbering a pushed value.
// ── Fragment color output ────────────────────────────────────────────────
out vec4 outColor;
void main() {
  vec3 N = normalize(vNormal);
  vec3 L = normalize(uLightPosition - vEyePos);   // surface → light
  vec3 V = normalize(-vEyePos);                   // surface → camera
  vec3 R = reflect(-L, N);                        // reflected light direction
  // Lambert diffuse — clamp to non-negative so unlit surfaces stay dark.
  float diffuse  = max(0.0, dot(N, L));
  // Phong specular — exponentiated alignment of reflected light with
  // view direction.
  float specular = pow(max(0.0, dot(R, V)), uSpecPower);
  // Compose: ambient + diffuse + specular. Specular is white (not
  // tinted by uColor) so the highlight reads as a light reflection.
  vec3 lit = uColor * (uAmbient + diffuse) + vec3(specular);
  outColor = vec4(lit, 1.0);
}