#version 300 es
precision highp float;
// ── Vertex-in (interpolated UVs) ─────────────────────────────────────────
in vec2 vTexCoord;
// ── Sampler uniform ──────────────────────────────────────────────────────
// A sampler2D is a handle to a texture unit, not the texture itself. The
// host binds a texture to a unit (e.g. TEXTURE0) and tells this uniform
// which unit to read from (an integer, typically 0).
uniform sampler2D uTex;
// ── Fragment color output ────────────────────────────────────────────────
out vec4 outColor;
void main() {
  // Passthrough sample: the row's job is to surface texture binding,
  // not to demonstrate a particular shader effect.
  outColor = texture(uTex, vTexCoord);
}