#version 300 es
precision highp float;
// ── Attributes (per-vertex inputs) ───────────────────────────────────────
in vec4 aPosition;
in vec3 aNormal;
// ── Uniforms (per-draw inputs) ───────────────────────────────────────────
// The three columns differ only in who fills these: a layer uploads the
// transforms its program declares, a column that writes the ceremony out
// pushes each one. The normal matrix is the inverse-transpose of the
// upper 3×3 of model-view — the right transform for normals when there's
// non-uniform scaling.
uniform mat4 uProjectionMatrix;
uniform mat4 uModelViewMatrix;
uniform mat3 uNormalMatrix;
// ── Vertex-out → fragment-in ─────────────────────────────────────────────
// Both passed through eye-space because the fragment shader's
// lighting calculation is in eye space.
out vec3 vNormal;
out vec3 vEyePos;
void main() {
  // Transform normal to eye space and normalize.
  vNormal = normalize(uNormalMatrix * aNormal);
  // Eye-space position — used by the fragment shader to compute the
  // view direction and the light direction.
  vec4 ePos = uModelViewMatrix * aPosition;
  vEyePos = ePos.xyz;
  // Standard projection to clip space.
  gl_Position = uProjectionMatrix * ePos;
}