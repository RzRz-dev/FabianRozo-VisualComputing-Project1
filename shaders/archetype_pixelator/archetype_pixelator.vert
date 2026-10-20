#version 300 es
precision highp float;
// ── Attributes (per-vertex inputs) ───────────────────────────────────────
// Positions in NDC directly — fullscreen quad covering the unit cube
// face at z=0, the same quad the texture row draws.
in vec4 aPosition;
in vec2 aTexCoord;
// ── Vertex-out → fragment-in ─────────────────────────────────────────────
out vec2 vTexCoord;
void main() {
  vTexCoord = aTexCoord;
  gl_Position = aPosition;
}