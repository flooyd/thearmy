"""A page that draws creatures with the shaders the game draws them with, in WebGL: the GLSL of each drawing (as shader.py makes it) with the game's own
HELPERS and fragment shader (read from src/creatures.rs), so what it shows is what the graphics card makes of them, and a drawing that would not compile says so
on the page, with the time it took. It is for checking art without starting the game (a browser can screenshot it, headless); it is not part of the game.

    python art/glpreview.py <page.html>                      every drawing
    python art/glpreview.py <page.html> --only Grunt,Titan   some, by their names in the game

    msedge --headless=new --use-angle=swiftshader --enable-unsafe-swiftshader --window-size=1600,1000 --screenshot=<shot.png> file:///<page.html>
"""

import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import shader  # noqa: E402
from dsl import REGISTRY  # noqa: E402

CREATURES = os.path.join(HERE, '..', 'src', 'creatures.rs')

#the vertex shader of the page: one square filling the part of the canvas a tile is given, the drawing on it facing up (the game's works out the same from an instance)
VERTEX = '''
attribute vec2 corner;
uniform float Pixels;
uniform vec4 Tint;
uniform float Hurt;
varying lowp vec4 color;
varying highp vec2 spot;
varying highp float aa;
varying highp float look;
varying highp float seed;
void main() {
    float screen = max(Pixels, 0.001);
    aa = SPAN / screen;
    spot = corner * vec2(1.0, -1.0) * SPAN;
    gl_Position = vec4(corner, 0.0, 1.0);
    color = Tint;
    look = Hurt;
    seed = 0.0;
}'''


def game_fragment():
    """The `main` of the game's fragment shader, as src/creatures.rs has it."""
    with open(CREATURES, encoding='utf-8') as fh:
        text = fh.read()
    return re.search(r'const FRAGMENT: &str = r#"(.*?)"#;', text, re.S).group(1)


def rgba(hexcolor):
    return [int(hexcolor[i:i + 2], 16) / 255.0 for i in (1, 3, 5)] + [1.0]


def page(syms):
    main = game_fragment()
    kinds = []
    for s in syms:
        plain = '0.0' if s.id.startswith('mark_') else '1.0'
        fragment = '#version 100\nprecision highp float;\nconst float SPAN = 80.0;\nconst float PLAIN = %s;\n%s\n%s\n%s' % (plain, shader.HELPERS, shader.function(s), main)
        kinds.append({'name': s.name, 'size': s.size, 'color': rgba(s.color), 'fragment': fragment})
    script = '''
const kinds = %s;
const vertex = "#version 100\\nconst float SPAN = 80.0;\\n" + %s;
const tile = 240, gap = 16;
const canvas = document.getElementById('c');
const across = Math.max(1, Math.floor((window.innerWidth - gap) / (tile * 1.9 + gap)));
const rows = Math.ceil(kinds.length / across);
canvas.width = across * (tile * 1.9 + gap) + gap;
canvas.height = rows * (tile + 40 + gap) + gap;
const gl = canvas.getContext('webgl', {premultipliedAlpha: true, alpha: false, antialias: false});
const notes = document.getElementById('notes');
gl.clearColor(0.075, 0.094, 0.12, 1.0);
gl.clear(gl.COLOR_BUFFER_BIT);
gl.enable(gl.BLEND);
gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
gl.enable(gl.SCISSOR_TEST);
const corners = gl.createBuffer();
gl.bindBuffer(gl.ARRAY_BUFFER, corners);
gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, 1, 1, -1, -1, 1, 1, -1, 1]), gl.STATIC_DRAW);
function compile(type, source) {
    const s = gl.createShader(type);
    gl.shaderSource(s, source);
    gl.compileShader(s);
    return s;
}
let total = 0;
kinds.forEach((kind, i) => {
    const col = i %% across, row = Math.floor(i / across);
    const x = gap + col * (tile * 1.9 + gap), top = gap + row * (tile + 40 + gap);
    const started = performance.now();
    const vs = compile(gl.VERTEX_SHADER, vertex), fs = compile(gl.FRAGMENT_SHADER, kind.fragment);
    const program = gl.createProgram();
    gl.attachShader(program, vs);
    gl.attachShader(program, fs);
    gl.linkProgram(program);
    const linked = gl.getProgramParameter(program, gl.LINK_STATUS);
    const ms = performance.now() - started;
    total += ms;
    const label = document.createElement('div');
    label.className = 'label';
    label.style.left = x + 'px';
    label.style.top = (top + tile + 4) + 'px';
    if (!linked) {
        label.innerHTML = '<b class="bad">' + kind.name + ' WOULD NOT COMPILE</b><pre>' + (gl.getShaderInfoLog(fs) || gl.getProgramInfoLog(program)) + '</pre>';
        notes.appendChild(label);
        return;
    }
    label.innerHTML = '<b>' + kind.name + '</b> ' + kind.size + ' px, ' + ms.toFixed(0) + ' ms';
    notes.appendChild(label);
    gl.useProgram(program);
    const at = gl.getAttribLocation(program, 'corner');
    gl.enableVertexAttribArray(at);
    gl.vertexAttribPointer(at, 2, gl.FLOAT, false, 0, 0);
    gl.uniform4fv(gl.getUniformLocation(program, 'Tint'), kind.color);
    gl.uniform1f(gl.getUniformLocation(program, 'Hurt'), 0.0);
    gl.uniform1f(gl.getUniformLocation(program, 'Time'), 1.25);
    //big, then as big as it is in the game zoomed all the way in to 1, then at half that
    const real = Math.round(kind.size * 1.6);
    [[x, tile], [x + tile + gap, real], [x + tile + gap, Math.round(real / 2)]].forEach(([left, px], k) => {
        const y = k == 2 ? top + Math.min(real, tile - 40) + 8 : top;
        gl.viewport(left, canvas.height - y - px, px, px);
        gl.scissor(left, canvas.height - y - px, px, px);
        gl.uniform1f(gl.getUniformLocation(program, 'Pixels'), px / 2);
        gl.drawArrays(gl.TRIANGLES, 0, 6);
    });
});
document.getElementById('total').textContent = kinds.length + ' drawings compiled in ' + total.toFixed(0) + ' ms';
''' % (json.dumps(kinds), json.dumps(VERTEX))
    return ('<!doctype html><html><head><meta charset="utf-8"><style>body{margin:0;background:#13181f;color:#cdd;font:13px system-ui}'
            '#wrap{position:relative}.label{position:absolute;width:420px}.bad{color:#ff6a5a}pre{white-space:pre-wrap;font-size:11px;color:#f99}'
            '#total{padding:6px 16px;color:#9ab}</style></head><body><div id="total"></div><div id="wrap"><canvas id="c"></canvas><div id="notes"></div></div>'
            '<script>%s</script></body></html>' % script)


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    only = None
    if '--only' in sys.argv:
        only = set(sys.argv[sys.argv.index('--only') + 1].split(','))
    syms = [s for s in REGISTRY if s.id not in shader.SKIP and (only is None or s.name in only)]
    with open(sys.argv[1], 'w', encoding='utf-8') as fh:
        fh.write(page(syms))
    print('%d drawings in %s' % (len(syms), sys.argv[1]))


if __name__ == '__main__':
    main()
