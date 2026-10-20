#version 300 es
precision highp float;
// ── Attributes (per-vertex inputs) ───────────────────────────────────────
// Vertices are supplied in NDC directly — the unit cube ([-1, 1]³) where
// the rasterizer expects them after the perspective divide. No view or
// projection transform is needed.
in vec4 aPosition;
in vec2 aTexCoord;
// ── Vertex-out → fragment-in ─────────────────────────────────────────────
out vec2 vTexCoord;
void main() {
  vTexCoord = aTexCoord;
  // Pass-through: aPosition is already in NDC.
  gl_Position = aPosition;
}