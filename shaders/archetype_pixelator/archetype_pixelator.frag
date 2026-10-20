#version 300 es
precision highp float;
// ── Vertex-in (interpolated UVs) ─────────────────────────────────────────
in vec2 vTexCoord;
// ── Uniforms ─────────────────────────────────────────────────────────────
// The scene texture rendered into an offscreen FBO. Sampled at
// snapped UVs to produce the pixelated effect.
uniform sampler2D uScene;
// Block size in normalized UV units' inverse: blocks across the canvas.
// At blocks = 16, every 16th of the canvas becomes a single colored
// block. Block centers are at (0.5 / blocks, 1.5 / blocks, ...).
uniform float uBlocks;
// ── Fragment color output ────────────────────────────────────────────────
out vec4 outColor;
void main() {
  // Snap to block, then sample from block center.
  // Without the + 0.5, sampling from block edges produces band
  // artifacts where the bilinear filter blends adjacent blocks.
  vec2 snapped = (floor(vTexCoord * uBlocks) + 0.5) / uBlocks;
  outColor = texture(uScene, snapped);
}