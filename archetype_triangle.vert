attribute vec3 aPosition;
attribute vec4 aVertexColor;
varying vec4 vVertexColor;
uniform mat4 uProjectionMatrix;
uniform mat4 uModelViewMatrix;
void main() {
    gl_Position = uProjectionMatrix * uModelViewMatrix * vec4(aPosition, 1.0);
    vVertexColor = aVertexColor;
}