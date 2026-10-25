"""Lays the drawings out as the artboards of a Design canvas, for looking at them (the mock-ups in the artifact; none of this is in the game).

    python art/mockups.py <folder>           writes <folder>/project/canvas.json and the .dc.html artboards
    python art/mockups.py <folder> --plain   and <folder>/plain/*.html: the same pages as ordinary html, to open in a browser
"""

import json
import math
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import build  # noqa: E402
from build import SPAN, VIEW  # noqa: E402
from dsl import REGISTRY, STYLE  # noqa: E402

BY_ID = {s.id: s for s in REGISTRY}
BOSS_IDS = ['hive_mother', 'iron_warden', 'warlord_grakk', 'eclipse', 'swarm_queen', 'singularity', 'bastion', 'sovereign']
MARK_IDS = [s.id for s in REGISTRY if s.id.startswith('mark_')]
ENEMY_IDS = [s.id for s in REGISTRY if s.id not in BOSS_IDS and s.id not in MARK_IDS and s.id not in ('ship', 'player')]

PHASES = {
    'hive_mother': ['the brood stirs', 'acid rain', 'frenzy'],
    'iron_warden': ['shields up', 'reinforced', 'overload'],
    'warlord_grakk': ['the charge', 'stampede', 'berserk'],
    'eclipse': ['the light fails', 'totality', 'oblivion'],
    'swarm_queen': ['the swarm wakes', 'she divides', 'a thousand wings'],
    'singularity': ['gravity gathers', 'event horizon', 'collapse'],
    'bastion': ['hold the line', 'crossfire', 'no retreat'],
    'sovereign': ['the court assembles', 'the crown burns', 'no mercy', 'the sovereign falls'],
}

INK, PANEL, EDGE, TEXT, MUTED, FLOOR = '#0d0f15', '#151922', '#232a38', '#e8ecf4', '#aab3c5', '#14141a'
HEAD = "'Chakra Petch', 'Segoe UI', sans-serif"
BODY = "'Inter', 'Segoe UI', sans-serif"
FONTS = '<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Chakra+Petch:wght@500;700&amp;family=Inter:wght@400;500;600&amp;display=swap">'


def closed(svg):
    """Every self-closing element written out with its closing tag, for a page that is not parsed as xml."""
    return re.sub(r'<([A-Za-z][A-Za-z0-9]*)([^<>]*?)\s*/>', r'<\1\2></\1>', svg)


def sprites(ids):
    symbols = ''.join('<symbol id="%s" viewBox="%s">%s</symbol>' % (i, VIEW, BY_ID[i].svg()) for i in ids)
    return closed('<svg xmlns="http://www.w3.org/2000/svg" width="0" height="0" aria-hidden="true" style="position:absolute">%s%s</svg>' % (build.defs(), symbols))


def esc(s):
    return s.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;')


def pic(id, px, color=None, marks=(), rot=0, hurt=False, hitbox=False, label=None, style=''):
    """A creature `px` across its whole picture (the hitbox is the middle 100/160 of it), in `color`, with `marks` (ids) over it and turned `rot` degrees."""
    s = BY_ID[id]
    color = color or s.color
    layers = '<use href="#%s" width="%d" height="%d"></use>' % (id, SPAN, SPAN)
    for m in marks:
        layers += '<g class="u" style="--c:%s"><use href="#%s" width="%d" height="%d"></use></g>' % (BY_ID[m].color, m, SPAN, SPAN)
    if hitbox:
        layers += '<rect x="%d" y="%d" width="100" height="100" fill="none" stroke="#ffffff" stroke-opacity="0.16" stroke-dasharray="3 3"></rect>' % ((SPAN - 100) // 2, (SPAN - 100) // 2)
    var = '--c:%s' % color + (';--v-b:inline' if hurt else '')
    turn = ';transform:rotate(%sdeg)' % rot if rot else ''
    name = 'role="img" aria-label="%s"' % esc(label or s.name)
    return ('<svg class="u" viewBox="0 0 %d %d" width="%s" height="%s" %s style="display:block;flex:none;%s%s%s">%s</svg>'
            % (SPAN, SPAN, round(px, 1), round(px, 1), name, var, turn, style, layers))


def page(title, w, h, body, ids, fonts=True):
    """One artboard: the .dc.html source."""
    css = 'body{margin:0}' + STYLE.replace('\n', '')
    return ('<!doctype html>\n<html lang="en">\n<head>\n<meta charset="utf-8">\n<title>%s</title>\n<script src="./support.js"></script>\n</head>\n<body>\n<x-dc>\n<helmet>\n%s\n<style>\n%s\n</style>\n</helmet>\n'
            '<div style="position:relative;width:%dpx;height:%dpx;box-sizing:border-box;overflow:hidden;background:%s;color:%s;font-family:%s">\n%s\n%s\n</div>\n</x-dc>\n'
            '<script type="text/x-dc" data-dc-script data-props=\'{"$preview":{"width":%d,"height":%d}}\'>\nclass Component extends DCLogic {\nrenderVals() {\nreturn {};\n}\n}\n</script>\n</body>\n</html>\n'
            % (title, FONTS if fonts else '', css, w, h, INK, TEXT, BODY, sprites(ids), body, w, h))


def head(eyebrow, title, text, width=760):
    return ('<div style="display:flex;flex-direction:column;gap:10px;max-width:%dpx">'
            '<div style="font-family:%s;font-weight:500;font-size:13px;letter-spacing:0.16em;text-transform:uppercase;color:#7fc8ff">%s</div>'
            '<h1 style="margin:0;font-family:%s;font-weight:700;font-size:52px;line-height:1.05">%s</h1>'
            '<p style="margin:0;font-size:16px;line-height:1.55;color:%s">%s</p></div>' % (width, HEAD, esc(eyebrow), HEAD, esc(title), MUTED, esc(text)))


def tag(text, color=MUTED, bg='#1d2330'):
    return '<span style="font-size:12px;line-height:1;padding:5px 8px;border-radius:5px;background:%s;color:%s;white-space:nowrap">%s</span>' % (bg, color, esc(text))


# ---------- the artboards

def enemies_board():
    w, cols, gap = 1440, 6, 16
    cards = []
    for id in ENEMY_IDS:
        s = BY_ID[id]
        cards.append(
            '<div style="height:280px;box-sizing:border-box;padding:12px;display:flex;flex-direction:column;gap:8px;background:%s;border:1px solid %s;border-radius:10px">'
            '<div style="height:176px;display:flex;align-items:center;justify-content:center;background:#0f1219;border-radius:7px">%s</div>'
            '<div style="display:flex;align-items:baseline;justify-content:space-between;gap:8px"><span style="font-family:%s;font-weight:700;font-size:17px">%s</span>%s</div>'
            '<div style="font-size:12.5px;line-height:1.4;color:%s">%s</div></div>'
            % (PANEL, EDGE, pic(id, 176, hitbox=True), HEAD, esc(s.name), tag('%d px' % s.size), MUTED, esc(s.role)))
    rows = math.ceil(len(cards) / cols)
    h = 56 + 190 + rows * 280 + (rows - 1) * gap + 64
    body = ('<div style="padding:56px 48px 0 48px;display:flex;flex-direction:column;gap:36px">%s'
            '<div style="display:grid;grid-template-columns:repeat(%d,minmax(0,1fr));gap:%dpx">%s</div></div>'
            % (head('The army · the horde', 'Twenty-three enemies',
                    'Each is drawn on its hitbox square (the dashed one) and outlined in black, as the ships are. The colour is the enemy\'s own: the shape stays, so a new colour is a new enemy.'),
               cols, gap, ''.join(cards)))
    return page('Enemies', w, h, body, ENEMY_IDS), w, h


def bosses_board():
    w, cols, gap = 1440, 4, 24
    cards = []
    for id in BOSS_IDS:
        s = BY_ID[id]
        chips = ''.join(tag(p) for p in PHASES[id])
        cards.append(
            '<div style="height:480px;box-sizing:border-box;padding:14px;display:flex;flex-direction:column;gap:10px;background:%s;border:1px solid %s;border-radius:12px">'
            '<div style="position:relative;height:300px;display:flex;align-items:center;justify-content:center;border-radius:8px;background:radial-gradient(circle at 50%% 48%%,%s2e,#0f1219 72%%)">%s'
            '<div style="position:absolute;right:8px;bottom:6px;display:flex;flex-direction:column;align-items:center;gap:2px">%s<span style="font-size:11px;color:%s">hurt</span></div></div>'
            '<div style="display:flex;align-items:baseline;justify-content:space-between;gap:8px"><span style="font-family:%s;font-weight:700;font-size:22px">%s</span>%s</div>'
            '<div style="font-size:13.5px;line-height:1.45;color:%s">%s</div>'
            '<div style="display:flex;flex-wrap:wrap;gap:6px">%s</div></div>'
            % (PANEL, EDGE, s.color, pic(id, 296, hitbox=False), pic(id, 84, hurt=True, label=s.name + ', hurt'), MUTED, HEAD, esc(s.name), tag('%d px' % s.size), MUTED, esc(s.role), chips))
    rows = math.ceil(len(cards) / cols)
    h = 56 + 190 + rows * 480 + (rows - 1) * gap + 64
    body = ('<div style="padding:56px 48px 0 48px;display:flex;flex-direction:column;gap:36px">%s'
            '<div style="display:grid;grid-template-columns:repeat(%d,minmax(0,1fr));gap:%dpx">%s</div></div>'
            % (head('The army · the bosses', 'Seven bosses and the Sovereign',
                    'Not bigger enemies: each is its own drawing, reaching past its hitbox. The small one in the corner is the same boss hurt, in the game\'s last stand.', 800),
               cols, gap, ''.join(cards)))
    return page('Bosses', w, h, body, BOSS_IDS), w, h


TINTS = [('crimson', '#e62938'), ('amber', '#f5a31e'), ('lemon', '#f5e61e'), ('lime', '#7fe633'), ('teal', '#1abf99'), ('azure', '#4d8ccc'), ('violet', '#a640f2'), ('magenta', '#f2339a')]
SPECIES = ['grunt', 'scout', 'brute', 'spitter', 'summoner', 'charger']


def colours_board():
    w, cell = 1440, 148
    label = 128
    head_row = ''.join('<div style="width:%dpx;text-align:center;font-size:12px;color:%s">%s</div>' % (cell, MUTED, n) for n, _ in TINTS)
    grid = ''
    for id in SPECIES:
        s = BY_ID[id]
        grid += ('<div style="display:flex;align-items:center;gap:0"><div style="width:%dpx;font-family:%s;font-weight:700;font-size:15px">%s</div>%s</div>'
                 % (label, HEAD, esc(s.name), ''.join('<div style="width:%dpx;height:%dpx;display:flex;align-items:center;justify-content:center">%s</div>' % (cell, cell, pic(id, 136, c)) for _, c in TINTS)))
    states = [('plain', ()), ('Swift', ('mark_swift',)), ('Armored', ('mark_armored',)), ('Volatile', ('mark_volatile',)), ('Frenzied', ('mark_frenzied',)),
              ('Captain', ('mark_captain',)), ('Frozen', ('mark_frozen',)), ('Shielded', ('mark_shield',))]
    sw = ''.join('<div style="width:152px;display:flex;flex-direction:column;align-items:center;gap:4px">%s<span style="font-size:13px;color:%s">%s</span></div>' % (pic('brute', 152, marks=m), MUTED, n) for n, m in states)
    h = 56 + 190 + 36 + 28 + len(SPECIES) * cell + 56 + 40 + 36 + 190 + 4
    body = ('<div style="padding:56px 48px 0 48px;display:flex;flex-direction:column;gap:36px">%s'
            '<div style="display:flex;flex-direction:column;gap:0"><div style="display:flex;align-items:center;margin-bottom:8px"><div style="width:%dpx"></div>%s</div>%s</div>'
            '<div style="display:flex;flex-direction:column;gap:14px;margin-top:20px"><div style="font-family:%s;font-weight:700;font-size:26px">Elites and states</div><div style="display:flex;gap:16px">%s</div></div></div>'
            % (head('The army · variety', 'The same shape, any colour',
                    'One drawing takes any tint: its body colour is the enemy\'s colour and its shades are worked out from that. So variety is mostly free; the shapes below are six of the twenty-three.', 820),
               label, head_row, grid, HEAD, sw))
    return page('Colours and marks', w, h, body, SPECIES + ['brute'] + MARK_IDS), w, h


def lineup_board():
    w, scale = 1440, 1.6 * 0.86

    def rows_of(ids):
        rows, row, used = [], [], 0
        for id in ids:
            px = BY_ID[id].size * scale
            if used + px + 28 > w - 96 and row:
                rows.append(row)
                row, used = [], 0
            row.append(id)
            used += px + 28
        return rows + [row]

    def line(ids):
        cells = ''.join('<div style="display:flex;flex-direction:column;align-items:center;gap:6px">%s<span style="font-size:11.5px;color:%s">%s</span></div>'
                        % (pic(i, BY_ID[i].size * scale), MUTED, esc(BY_ID[i].name)) for i in ids)
        return '<div style="display:flex;align-items:flex-end;justify-content:space-between;gap:14px">%s</div>' % cells

    enemy_rows = rows_of(sorted(ENEMY_IDS, key=lambda i: BY_ID[i].size))
    boss_rows = rows_of(sorted(BOSS_IDS, key=lambda i: BY_ID[i].size))
    hero = ('<div style="display:flex;align-items:flex-end;gap:18px"><div style="display:flex;flex-direction:column;align-items:center;gap:6px">%s<span style="font-size:11.5px;color:%s">Player</span></div>'
            '<div style="display:flex;flex-direction:column;align-items:center;gap:6px">%s<span style="font-size:11.5px;color:%s">Ship</span></div></div>'
            % (pic('player', 44 * scale * 0.7), MUTED, pic('ship', 30 * scale), MUTED))
    row_h = [max(BY_ID[i].size for i in r) * scale + 30 for r in enemy_rows + boss_rows]
    gap = 28
    h = int(56 + 190 + 60 + sum(row_h) + gap * (len(row_h) + 3) + 40)
    body = ('<div style="padding:56px 48px 0 48px;display:flex;flex-direction:column;gap:%dpx">%s%s%s%s%s</div>'
            % (gap, head('The army · scale', 'Everyone at their size',
                         'Every creature drawn at its in-game size, smallest to biggest, to see which can still be told apart small. A ship and the player stand beside them; the bosses are underneath.', 800),
               hero, ''.join(line(r) for r in enemy_rows), '<div style="height:1px;background:%s"></div>' % EDGE, ''.join(line(r) for r in boss_rows)))
    return page('Lineup', w, h, body, ENEMY_IDS + BOSS_IDS + ['player', 'ship']), w, h


def field_board():
    w, h = 1440, 900
    px_at = (700, 560)
    items = []   # (id, x, y, px, color, marks, rotate)

    def add(id, x, y, color=None, marks=(), turn=True, scale=1.6, hurt=False):
        s = BY_ID[id]
        rot = 0
        if turn:
            rot = round(math.degrees(math.atan2(px_at[0] - x, -(px_at[1] - y))))
        items.append((id, x, y, s.size * scale, color, marks, rot, hurt))

    # the army: a ring of ships round the player
    for k in range(20):
        a = math.radians(k * 18 + 4)
        items.append(('ship', px_at[0] + 78 * math.sin(a), px_at[1] - 78 * math.cos(a), 38, '#5aa9ff', (), round(k * 18 + 4), False))
    for k in range(10):
        a = math.radians(k * 36 + 22)
        items.append(('ship', px_at[0] + 44 * math.sin(a), px_at[1] - 44 * math.cos(a), 38, '#7fc2ff', (), round(k * 36 + 22), False))
    items.append(('player', px_at[0], px_at[1], 48, '#e8eef7', (), 0, False))

    # the west: a captain and the horde it leads
    add('brute', 290, 500, '#a640f2', ('mark_captain',), turn=False)
    for x, y, c in ((170, 410, '#e62938'), (230, 360, '#e62938'), (210, 580, '#e62938'), (140, 520, '#f5a31e'), (330, 380, '#e62938'), (360, 610, '#f5a31e'), (410, 470, '#e62938'), (120, 640, '#e62938')):
        add('grunt', x, y, c)
    for x, y in ((470, 360), (500, 410), (440, 420), (520, 340), (480, 480)):
        add('swarmer', x, y, '#99e633')
    for x, y in ((410, 560), (455, 585)):
        add('scout', x, y, '#f5e61e', ('mark_swift',))
    add('bomber', 540, 610, '#666675', ('mark_volatile',))

    # the north: the Iron Warden with its shield, and a nest with its brood
    add('iron_warden', 760, 190, marks=('mark_shield',), turn=False)
    add('nest', 150, 160, '#cc404d', turn=False)
    for x, y in ((250, 120), (270, 210), (190, 270), (95, 270)):
        add('imp', x, y, '#cc4de6')
    add('summoner', 390, 215, '#5940bf')
    add('spitter', 1010, 290, '#9e6b40')

    # the north east: a bastion standing off, with what it sends
    add('bastion', 1230, 150, turn=False)
    add('lancer', 1090, 430, '#d9ebff')
    add('mortar', 1300, 340, '#8c9933', turn=False)
    add('seeker', 960, 130, '#eb4db8', turn=False)

    # the east: a charger on its lane, with a healer and an armoured bulwark behind
    add('charger', 1050, 600, '#992e1f', ('mark_frenzied',))
    add('bulwark', 1190, 640, '#4d8ccc', ('mark_armored',), turn=False)
    add('healer', 1290, 520, '#99ffb3', turn=False)
    add('grenadier', 1180, 480, '#e68c1a')
    add('drone', 880, 400, '#80ccff', turn=False)
    add('drone', 930, 450, '#80ccff', turn=False)
    add('drone', 840, 460, '#80ccff', turn=False)

    # the south: slow big ones, frozen and splitting
    add('titan', 560, 790, '#f2801a', turn=False)
    add('colossus', 1250, 800, '#f2339a', turn=False, scale=1.0)
    add('splitter', 900, 770, '#1abf99', turn=False)
    add('splitling', 960, 700, '#59e6bf', turn=False)
    add('splitling', 850, 690, '#59e6bf', turn=False)
    add('grunt', 760, 720, '#e62938', ('mark_frozen',), turn=False)
    add('ambusher', 340, 760, '#5c4275')
    add('ambusher', 430, 800, '#5c4275')

    placed = ''
    for id, x, y, px, color, marks, rot, hurt in items:
        placed += ('<div style="position:absolute;left:%dpx;top:%dpx;width:%dpx;height:%dpx">%s</div>'
                   % (x - px / 2, y - px / 2, px, px, pic(id, px, color, marks, rot, hurt)))

    # what is on the ground: a danger circle, a charge lane, an acid pool, the boss's next circle, the army's shots
    ground = (
        '<svg width="%d" height="%d" viewBox="0 0 %d %d" aria-hidden="true" style="position:absolute;left:0;top:0">'
        '<circle cx="560" cy="330" r="86" fill="#ff3a3a" fill-opacity="0.14" stroke="#ff5a4a" stroke-opacity="0.7" stroke-width="3"></circle>'
        '<line x1="1050" y1="600" x2="770" y2="565" stroke="#ff3a26" stroke-opacity="0.2" stroke-width="62" stroke-linecap="round"></line>'
        '<line x1="1050" y1="600" x2="770" y2="565" stroke="#ff7355" stroke-opacity="0.7" stroke-width="4" stroke-linecap="round"></line>'
        '<circle cx="1010" cy="330" r="64" fill="#8dff6a" fill-opacity="0.12"></circle>'
        '<circle cx="760" cy="190" r="228" fill="none" stroke="#7399d9" stroke-opacity="0.25" stroke-width="5"></circle>'
        '<path d="M760 -38 A228 228 0 0 1 960 76" fill="none" stroke="#ffa83a" stroke-opacity="0.9" stroke-width="7" stroke-linecap="round" transform="translate(0 0)"></path>'
        '<circle cx="1290" cy="520" r="120" fill="none" stroke="#8dff6a" stroke-opacity="0.35" stroke-width="3" stroke-dasharray="6 8"></circle>'
        '<path d="M1172 150 A84 84 0 0 0 1230 232" fill="none" stroke="#bfe6ff" stroke-opacity="0.9" stroke-width="6" stroke-linecap="round"></path>'
        '</svg>' % (w, h, w, h))
    shots = ''
    for x, y, a in ((660, 470, 332), (750, 468, 18), (620, 520, 300), (790, 520, 62), (700, 455, 358), (640, 480, 318)):
        shots += '<div style="position:absolute;left:%dpx;top:%dpx;width:5px;height:22px;margin:-11px 0 0 -2.5px;border-radius:3px;background:#69f0ff;box-shadow:0 0 8px #69f0ff;transform:rotate(%ddeg)"></div>' % (x, y, a)
    for x, y in ((1150, 250), (1120, 290), (1000, 380), (930, 330)):
        shots += '<div style="position:absolute;left:%dpx;top:%dpx;width:12px;height:12px;margin:-6px 0 0 -6px;border-radius:50%%;background:#ffa83a;border:2px solid #06090d;box-shadow:0 0 10px #ff7a2a"></div>' % (x, y)
    floor = ('<div style="position:absolute;left:0;top:0;width:%dpx;height:%dpx;background-color:%s;'
             'background-image:linear-gradient(rgba(255,255,255,0.05) 1px,transparent 1px),linear-gradient(90deg,rgba(255,255,255,0.05) 1px,transparent 1px);background-size:64px 64px;background-position:20px 12px"></div>'
             % (w, h, FLOOR))
    caption = ('<div style="position:absolute;left:28px;bottom:24px;max-width:430px;display:flex;flex-direction:column;gap:4px"><span style="font-family:%s;font-weight:700;font-size:22px">In the field</span>'
               '<span style="font-size:13px;color:%s">A mock-up at game scale: an army of thirty ships and the player, a captain\'s horde, a boss with its shield, a charge, a danger circle and a bastion out of reach.</span></div>' % (HEAD, MUTED))
    ids = sorted({i[0] for i in items} | set(MARK_IDS))
    body = floor + ground + shots + placed + caption
    return page('In the field', w, h, body, ids), w, h


BOARDS = [('Main.dc.html', 'Enemies', enemies_board), ('Bosses.dc.html', 'Bosses', bosses_board), ('Lineup.dc.html', 'Everyone at their size', lineup_board),
          ('Colours.dc.html', 'Colours and marks', colours_board), ('Field.dc.html', 'In the field', field_board)]


def plain(src):
    """The artboard as an ordinary page: the runtime's own tags taken out, to look at it in a browser."""
    src = src.replace('<script src="./support.js"></script>', '').replace('<x-dc>', '').replace('</x-dc>', '').replace('<helmet>', '').replace('</helmet>', '')
    return re.sub(r'<script type="text/x-dc".*?</script>', '', src, flags=re.S)


def main():
    out = sys.argv[1]
    os.makedirs(os.path.join(out, 'project'), exist_ok=True)
    made = []
    for name, title, fn in BOARDS:
        src, w, h = fn()
        assert '{{' not in src, name
        with open(os.path.join(out, 'project', name), 'w', encoding='utf-8', newline='\n') as f:
            f.write(src)
        if '--plain' in sys.argv:
            os.makedirs(os.path.join(out, 'plain'), exist_ok=True)
            with open(os.path.join(out, 'plain', name.replace('.dc.html', '.html')), 'w', encoding='utf-8') as f:
                f.write(plain(src))
        made.append((name, title, w, h))
        print('%s %dx%d %dKB' % (name, w, h, len(src) // 1024))
    # two rows, each under a title: the roster and the bosses, then the others. a title is 260 above its row, and a row is 120 under the last
    layout = [([0, 1], 'The horde and the bosses'), ([2, 3, 4], 'Variety, scale and the field')]
    boards, order, notes, y = {}, [], {}, 0
    for r, (row, title) in enumerate(layout):
        x = 0
        y += 260 if r else 0
        notes['t%d' % (r + 1)] = {'x': 0, 'y': y - 260 if r else -260, 'text': title, 'kind': 'title1', 'maxW': sum(made[i][2] for i in row) + 80 * (len(row) - 1)}
        for i in row:
            name, label, w, h = made[i]
            boards[name] = {'x': x, 'y': y, 'w': w, 'h': h, 'title': label}
            order.append(name)
            x += w + 80
        y += max(made[i][3] for i in row) + 120
    index = {'v': 3, 'createdOnFiles': {'v': 1, 'at': '2026-10-24T21:00:00Z'}, 'title': 'Enemy and Boss Art', 'launch': {'view': 'canvas'}, 'pages': [], 'boards': boards, 'order': order, 'notes': notes, 'designSystems': []}
    with open(os.path.join(out, 'project', 'canvas.json'), 'w', encoding='utf-8', newline='\n') as f:
        json.dump(index, f, indent=1)


if __name__ == '__main__':
    main()
