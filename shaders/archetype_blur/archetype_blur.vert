#version 300 es
precision highp float;
// archetype_blur.vert — fullscreen NDC pass-through, shared by the three
// columns. Positions arrive already in clip space; UVs ride along to the
// fragment stage. No view, no projection — a post pass owns the screen.
in vec4 aPosition;
in vec2 aTexCoord;
out vec2 vTexCoord;
void main() {
  vTexCoord = aTexCoord;
  gl_Position = aPosition;
}