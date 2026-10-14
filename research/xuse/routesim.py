"""Route simulation, both sides: the original's bytecode and the converted 0.txt.

usage: python3 -I routesim.py <game dir> <0.txt> <path> [--lore] [--show]

Runs one click path through each side and diffs the event streams: chapters,
text (compared as blocks between other events), the links each page offers,
the links clicked and where they go, and the progression variables at each
「お茶会－０」 (the start of a loop). docs/new-game.md step 11.4.

The source side follows the original engine's run-time rules: link, ruby,
style and indent records collect until a message uses them, so a jump past
a record leaves it out. Variables: globals (0x8000) are %200+n, locals
%10+n, as in the conversion.

<path> names a click path in PATHS, one entry per loop: the link text to
click for the route (None for no link), plus texts to try anyway. --lore
also clicks every other link once per loop (not the heroine links).
"""
import sys, os, re, subprocess, struct, collections, difflib, hashlib

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import cdlib

# The four heroine links of 「最初の日」 (0004.cd), in text order.
AZUSA, SAYAKA, FUEKO, YOUKO = '最初の頃', 'たまに早く出る', '違う制服', '静かな朝'
HEROINES = {AZUSA, SAYAKA, FUEKO, YOUKO}
# One entry per loop: (route link, other texts to try in that loop).
PATHS = {
    # The walkthrough (docs/games/saihate-no-ima.md, "Walkthrough"): no link, the four
    # heroines, no link, the four again, then the war arc.
    'walkthrough': [(None, ()), (AZUSA, ()), (SAYAKA, ()), (FUEKO, ()), (YOUKO, ()),
                    (None, ()), (None, ()), (AZUSA, ()), (SAYAKA, ()), (FUEKO, ()), (YOUKO, ()), (None, ()), (None, ())],
    # The user's bug: Azusa's link, then Sayaka's in the same loop.
    'two-heroines': [(AZUSA, (SAYAKA, FUEKO, YOUKO))],
    'no-links': [(None, ())] * 4,
}
CHAPTER_LOOP = 'お茶会－０'
PROGRESS_VARS = list(range(200, 221))
MAX_STEPS = 40_000_000


class Stop(Exception):
    pass


class Policy:
    """Decides which links to click; the same object type drives both sides."""
    def __init__(self, loops, lore):
        self.loops, self.lore = loops, lore
        self.loop = -1          # index into loops; 0 after the first お茶会－０
        self.clicked = set()    # link texts clicked this loop

    def new_loop(self):
        self.loop += 1
        self.clicked = set()
        if self.loop >= len(self.loops):
            raise Stop(f'path done after {len(self.loops)} loops')

    def choose(self, texts):
        """Index of the link to click on a page offering `texts`, or None."""
        route, extra = self.loops[self.loop] if self.loop >= 0 else (None, ())
        for i, t in enumerate(texts):
            if t in self.clicked:
                continue
            if t == route or t in extra or (self.lore and t not in HEROINES):
                self.clicked.add(t)
                return i
        return None


BLOCKS = {}  # text block hash -> text


def text_diffs(diff, limit=40):
    """The characters that differ between adjacent -TEXT/+TEXT lines."""
    out, prev = collections.Counter(), None
    for d in diff:
        h = re.search(r', ([0-9a-f]{8})\]$', d)
        if d.startswith('-TEXT') and h:
            prev = BLOCKS[h.group(1)]
        elif d.startswith('+TEXT') and h and prev is not None:
            cur = BLOCKS[h.group(1)]
            for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, prev, cur, autojunk=False).get_opcodes():
                if tag != 'equal':
                    out[(prev[max(0, i1 - 6):i2 + 6], cur[max(0, j1 - 6):j2 + 6])] += 1
            prev = None
        else:
            prev = None
    return out.most_common(limit)


class Events:
    def __init__(self, show):
        self.ev, self.text, self.show = [], [], show

    def flush(self):
        if self.text:
            block = ''.join(self.text)
            BLOCKS[hashlib.md5(block.encode()).hexdigest()[:8]] = block
            self.ev.append(f'TEXT {block[:40]}… [{len(block)} chars, {hashlib.md5(block.encode()).hexdigest()[:8]}]')
            self.text = []

    def page(self, text):
        self.text.append(text)

    def add(self, e):
        self.flush()
        self.ev.append(e)
        if self.show:
            print('   ', e)


# The conversion writes these ASCII characters full-width (they mean ruby
# or commands in NScripter text); compare both sides that way.
FULL_WIDTH = str.maketrans('()/!_;*', '（）／！＿；＊')


def norm(s):
    return re.sub(r'[\s　]', '', s).translate(FULL_WIDTH)


# ---------------------------------------------------------------- source side

def tool(game, *args):
    exe = os.path.join(HERE, '..', '..', 'target', 'release', 'conscripter-convert')
    return subprocess.run([exe, *args], check=True, capture_output=True, text=True).stdout


def load_source(game):
    d = os.path.join(game, 'HD', 'CDFiles')
    scripts = {}
    for f in sorted(os.listdir(d)):
        if not re.fullmatch(r'\d{4}\.cd', f):
            continue
        path = os.path.join(d, f)
        dump = tool(game, 'cd', 'dump', path)
        sid = int(re.search(r'\(id (\d+)\)', dump).group(1))
        labels = [tuple(map(int, m.groups())) for m in re.finditer(r'^label\s+(\d+)\s+(\d+)\s+(\d+)$', dump, re.M)]
        code = [(int(m.group(2), 16), int(m.group(3), 16), int(m.group(4))) for m in
                re.finditer(r'^\s+(\d+)\s+op (0x[0-9a-f]+)\s+sub (0x[0-9a-f]+)\s+arg\s+(-?\d+)', dump, re.M)]
        code = [(op, sub, arg) for op, sub, arg in code]
        msgs, links = {}, {}
        cur = None
        for line in tool(game, 'cd', 'messages', path).splitlines():
            m = re.match(r'#(\d+) link row (\d+) col (\d+) len (\d+) style \S+ target (\d+)', line)
            if m:
                links[int(m.group(1))] = tuple(map(int, m.groups()[1:]))
                cur = None
                continue
            m = re.match(r'#(\d+) line', line)
            if m:
                cur = msgs.setdefault(int(m.group(1)), [])
                continue
            if cur is not None and line.startswith('    '):
                cur.append(line[4:])
        _, _, pool = cdlib.load(path)
        scripts[sid] = dict(labels=labels, code=code, msgs=msgs, links=links, pool=pool)
    syms = collections.defaultdict(dict)  # script -> label -> name
    for line in tool(game, 'cd', 'symbols', os.path.join(d, '0.cd')).splitlines():
        m = re.match(r'script (\d+):', line)
        if m:
            sid = int(m.group(1))
            continue
        m = re.match(r'\s+\d+\s+\[(\d+), (\d+), (\d+)\]\s+(.*)$', line)
        if m:
            syms[sid][int(m.group(2))] = m.group(4)
    for sid, s in scripts.items():
        s['own'] = {lab: off + 1 for scr, lab, off in s['labels'] if scr == sid}
        s['syms'] = syms.get(sid, {})
    return scripts


def u32(pool, o):
    return struct.unpack_from('<I', pool, o)[0]


def var_of(code):
    return (200 if code & 0x8000 else 10) + (code & 0x7fff)


def chapter_title(name):
    if not name or not name.startswith('chapter') or ']' not in name:
        return None
    rest = name.split(']', 1)[1]
    return rest.split('?', 1)[1] if '?' in rest else None


def run_source(scripts, policy, ev):
    var = collections.defaultdict(int)
    stack = []      # (script, index, is_link_scene)
    pending = []    # link records waiting for a message
    linkpage = []   # per open link scene: (page links, return point) to go on clicking
    def table(s, n):
        if n < 1 or n > len(s['labels']):
            return None
        scr, lab, _ = s['labels'][n - 1]
        return scr, scripts[scr]['own'][lab]
    sid, pc = 0, scripts[0]['own'][1]
    steps = 0
    try:
        while True:
            steps += 1
            if steps > MAX_STEPS:
                raise Stop('step limit')
            s = scripts[sid]
            if pc >= len(s['code']):
                raise Stop(f'ran off the end of script {sid}')
            op, sub, arg = s['code'][pc]
            nxt = pc + 1
            if op in (0x46,):
                pending.append(s['links'][pc])
            elif op in (0x40, 0x41):
                segs = s['msgs'].get(pc, [])
                speaker = len(segs) >= 2 and 1 <= len(segs[0]) <= 8 and segs[1][:1] in '「『（'
                body = segs[1:] if speaker else segs
                ev.page(norm(''.join(body)))
                offered = []
                for row, col, length, target in pending:
                    r, c = row - 1, col - 1
                    if r < int(speaker) or r >= len(segs) or c < 0 or c + length > len(segs[r]):
                        continue
                    offered.append((norm(segs[r][c:c + length]), target))
                pending = []
                if offered:
                    ev.add('LINKS ' + ' | '.join(t for t, _ in offered))
                    k = policy.choose([t for t, _ in offered])
                    if k is not None:
                        dest = table(s, offered[k][1])
                        ev.add(f'CLICK {offered[k][0]} -> s{dest[0]}_l{s["labels"][offered[k][1] - 1][1]}' if dest else f'CLICK {offered[k][0]} -> ?')
                        if dest:
                            linkpage.append((offered, sid, pc))
                            stack.append((sid, nxt, True))
                            sid, pc = dest
                            continue
            elif op == 0x44 or op == 0x49 or op == 0x4e:
                pass  # other records: they don't steer the story
            elif op in (0x10, 0x11):
                if arg in s['own']:
                    nxt = s['own'][arg]
            elif op == 0x15:
                dest = table(s, sub)
                if not dest:
                    raise Stop(f'goto to an unresolved label (script {sid} #{pc})')
                sid, pc = dest
                continue
            elif 0x16 <= op <= 0x1b:
                o = arg & 0xffffffff
                a, b = var[var_of(u32(s['pool'], o))], struct.unpack_from('<i', s['pool'], o + 4)[0]
                if [a == b, a != b, a < b, a > b, a <= b, a >= b][op - 0x16]:
                    dest = table(s, sub)
                    if dest:
                        sid, pc = dest
                        continue
            elif 0x01 <= op <= 0x0e:
                t = var_of(sub)
                by_var = op >= 0x08 or op == 0x03
                v = var[var_of(arg & 0xffff)] if by_var else arg
                kind = {1: 'set', 2: 'add', 3: 'sub', 4: 'mul', 5: 'div', 6: 'and', 7: 'or', 8: 'set', 9: 'add',
                        0xa: 'sub', 0xb: 'mul', 0xc: 'div', 0xd: 'and', 0xe: 'or'}[op]
                var[t] = {'set': v, 'add': var[t] + v, 'sub': var[t] - v, 'mul': var[t] * v,
                          'div': int(var[t] / v) if v else 0, 'and': var[t] & v, 'or': var[t] | v}[kind]
            elif op == 0x22:
                dest = table(s, sub)
                if dest:
                    stack.append((sid, nxt, False))
                    sid, pc = dest
                    continue
            elif op in (0x23, 0x47):
                if not stack:
                    raise Stop('return with an empty stack')
                sid, pc, was_link = stack.pop()
                if was_link:
                    offered, psid, ppc = linkpage.pop()
                    k = policy.choose([t for t, _ in offered])
                    if k is not None:
                        ps = scripts[psid]
                        dest = table(ps, offered[k][1])
                        ev.add(f'CLICK {offered[k][0]} -> s{dest[0]}_l{ps["labels"][offered[k][1] - 1][1]}')
                        linkpage.append((offered, psid, ppc))
                        stack.append((sid, pc, True))
                        sid, pc = dest
                continue
            elif op == 0x48:
                # Leave every open link scene; at the root, go on after the 0x48.
                while any(l for _, _, l in stack):
                    _, _, l = stack.pop()
                    if l:
                        linkpage.pop()
            elif op == 0x36:
                title = chapter_title(s['syms'].get(u32(s['pool'], arg & 0xffffffff)))
                if title:
                    chapter(title, var, policy, ev)
            elif op == 0x96:
                raise Stop('back to the title')
            pc = nxt
    except Stop as e:
        ev.add(f'STOP {e}')
        ev.add(vars_line(var))


def chapter(title, var, policy, ev):
    # Spaces don't count: sprite text pads odd runs of half-width
    # characters with one (conv-cons, sprite_text).
    ev.add(f'CHAPTER {norm(title)}')
    if norm(title) == CHAPTER_LOOP:
        ev.add(vars_line(var))
        policy.new_loop()
        ev.add(f'LOOP {policy.loop + 1}')


def vars_line(var):
    return 'VARS ' + ' '.join(f'{v}={var[v]}' for v in PROGRESS_VARS if var[v])


# ------------------------------------------------------------- converted side

def split_cmds(line):
    out, cur, q = [], '', False
    for c in line:
        if c == '"':
            q = not q
        if c == ':' and not q:
            out.append(cur)
            cur = ''
        else:
            cur += c
    out.append(cur)
    return [c.strip() for c in out]


def plain(text):
    text = re.sub(r'^\[[^\]|]*\|[^\]]*\]', '', text)
    text = re.sub(r'#[0-9A-Fa-f]{6}', '', text)
    text = re.sub(r'\(([^/()]*)/[^()]*\)', r'\1', text)
    text = re.sub(r'!(z\d+|s\d+|sd|w\d+|d\d+)', '', text)
    # A window without a name plate shows the speaker inline (conv-cons).
    text = re.sub(r'【[^】]{1,8}】(?=[「『（])', '', text)
    return norm(text.replace('\\', '').replace('@', ''))


def run_converted(path, policy, ev):
    lines = open(path, 'rb').read().decode('cp932').split('\n')
    lines = [l.rstrip('\r') for l in lines]
    labels = {l[1:].strip(): i for i, l in enumerate(lines) if l.startswith('*')}
    var = collections.defaultdict(int)
    svar = collections.defaultdict(str)
    stack = []
    buttons, sprite_text = {}, {}
    again = [False]      # a jump back to a link page: it is shown again
    after_click = [False]
    fresh = [False]      # a page was shown since the last btnwait
    text = []

    def num(t):
        t = t.strip()
        if re.fullmatch(r'-?\d+', t):
            return int(t)
        if t.startswith('%%'):
            return var[var[int(t[2:])]]
        e = re.sub(r'%%(\d+)', lambda m: str(var[var[int(m.group(1))]]), t)
        e = re.sub(r'%(\d+)', lambda m: str(var[int(m.group(1))]), e)
        if re.fullmatch(r'[-+*/() \d]+', e):
            return int(eval(e.replace('/', '//')))
        return 0

    def sval(t):
        t = t.strip()
        if t.startswith('"'):
            return t[1:-1]
        if t.startswith('$%'):
            return svar[var[int(t[2:])]]
        if t.startswith('$'):
            return svar[int(t[1:])]
        return ''

    def cond(c):
        for part in c.split('&&'):
            m = re.match(r'\s*(\S+)\s*(==|!=|>=|<=|>|<)\s*(\S+)\s*$', part)
            if not m:
                return False
            a, o, b = m.groups()
            if a.startswith('$'):
                x, y = sval(a), sval(b)
            else:
                x, y = num(a), num(b)
            if not {'==': x == y, '!=': x != y, '>=': x >= y, '<=': x <= y, '>': x > y, '<': x < y}[o]:
                return False
        return True

    def var_ref(t):
        t = t.strip()
        return var[int(t[2:])] if t.startswith('%%') else int(t[1:])

    def flush_text():
        if not text:
            return
        page = ''.join(text)
        text.clear()
        if again[0]:
            again[0] = False   # the link page shown again after its scene
            fresh[0] = False
            return
        ev.page(page)
        fresh[0] = True
        sprite_text.clear()

    def jump(name):
        name = name.strip().lstrip('*')
        # The page falls through its label the first time and is jumped back
        # to after a link scene (conv-cons link pages).
        if re.fullmatch(r'conv_link\d+', name):
            again[0] = True
        if after_click[0] and re.fullmatch(r's\d+_l\d+', name):
            ev.add(f'-> {name}')
            after_click[0] = False
        return labels[name]

    pc = labels['conv_title_start'] + 1
    resume = 0           # command index to go on at (after a return)
    steps = 0
    try:
        while True:
            steps += 1
            if steps > MAX_STEPS:
                raise Stop('step limit')
            if pc >= len(lines):
                raise Stop('ran off the end')
            l = lines[pc]
            if not l or l.startswith((';', '*')):
                pc += 1
                continue
            if not re.match(r'[a-z]', l):
                text.append(plain(l))
                if l.endswith('\\'):
                    flush_text()
                pc += 1
                continue
            flush_text()
            new = None
            cmds = split_cmds(l)
            i, resume = resume, 0
            while i < len(cmds):
                cmd = cmds[i]
                i += 1
                m = re.match(r'if (.+?) ((?:goto|gosub|return|mov|add|sub|mul|div|mod|inc|dec|strsp|spbtn|btndef|reset|end)\b.*)$', cmd)
                if m:
                    if not cond(m.group(1)):
                        break
                    cmd = m.group(2)
                w = cmd.split(' ', 1)
                op, rest = w[0], (w[1] if len(w) > 1 else '')
                if op == 'goto':
                    new = jump(rest)
                elif op == 'gosub':
                    # A return goes on with the rest of this line.
                    stack.append((pc, i) if i < len(cmds) else (pc + 1, 0))
                    new = jump(rest)
                elif op == 'return':
                    if not stack:
                        raise Stop('return with an empty stack')
                    new, resume = stack.pop()
                elif op in ('mov', 'add', 'sub', 'mul', 'div', 'mod'):
                    a, b = rest.split(',', 1)
                    a = a.strip()
                    if a.startswith('$'):
                        k = var[int(a[2:])] if a.startswith('$%') else int(a[1:])
                        svar[k] = sval(b)
                        if k == 150 and svar[k]:
                            chapter(svar[k], var, policy, ev)
                        continue
                    if not a.startswith('%'):
                        continue
                    k, v = var_ref(a), num(b)
                    var[k] = {'mov': v, 'add': var[k] + v, 'sub': var[k] - v, 'mul': var[k] * v,
                              'div': int(var[k] / v) if v else 0, 'mod': var[k] % v if v else 0}[op]
                elif op in ('inc', 'dec'):
                    k = var_ref(rest)
                    var[k] += 1 if op == 'inc' else -1
                elif op == 'btndef':
                    buttons.clear()
                elif op == 'spbtn':
                    n, k = rest.split(',')
                    buttons[int(k)] = int(n)
                elif op == 'strsp':
                    n, t = rest.split(',', 2)[:2]
                    sprite_text[int(n)] = t.strip().strip('"')
                elif op in ('btnwait', 'textbtnwait'):
                    links = sorted(k for k in buttons if 1 <= k <= 1000)
                    choice = 0
                    if op == 'btnwait' and links:
                        texts = [norm(sprite_text.get(buttons[k], '?')) for k in links]
                        if fresh[0]:
                            ev.add('LINKS ' + ' | '.join(texts))
                            fresh[0] = False
                        k = policy.choose(texts)
                        if k is not None:
                            choice = links[k]
                            ev.add(f'CLICK {texts[k]}')
                            after_click[0] = True
                    var[var_ref(rest)] = choice
                elif op in ('isskip', 'getcursorpos', 'getmousepos'):
                    for a in rest.split(','):
                        if a.strip().startswith('%'):
                            var[var_ref(a)] = 0
                elif op == 'gettimer':
                    var[var_ref(rest)] = steps * 10
                elif op == 'reset':
                    raise Stop('back to the title')
                elif op == 'end':
                    raise Stop('end')
                if new is not None:
                    break
            pc = new if new is not None else pc + 1
    except Stop as e:
        flush_text()
        ev.add(f'STOP {e}')
        ev.add(vars_line(var))


last_page = ['']


def main():
    game, txt, name = sys.argv[1:4]
    lore, show = '--lore' in sys.argv, '--show' in sys.argv
    print(f'path {name}{" with lore links" if lore else ""}')
    print('source …', flush=True)
    src = Events(show)
    run_source(load_source(game), Policy(PATHS[name], lore), src)
    print('converted …', flush=True)
    conv = Events(show)
    # The converted side learns a link page's text before its buttons.
    orig_page = conv.page
    def page(t):
        last_page[0] = t
        orig_page(t)
    conv.page = page
    run_converted(txt, Policy(PATHS[name], lore), conv)
    src.flush(); conv.flush()
    a = [e for e in src.ev if not e.startswith('-> ')]
    # Converted CLICK + "-> target" pairs become the source's form.
    b, i = [], 0
    while i < len(conv.ev):
        e = conv.ev[i]
        if e.startswith('CLICK ') and i + 1 < len(conv.ev) and conv.ev[i + 1].startswith('-> '):
            b.append(f'{e} {conv.ev[i + 1]}')
            i += 2
            continue
        b.append(e if not e.startswith('CLICK ') else e + ' -> ?')
        i += 1
    print(f'source: {len(a)} events, converted: {len(b)} events')
    for e in a:
        if e.startswith(('CHAPTER', 'LOOP', 'STOP', 'VARS')):
            print('  ', e)
    diff = list(difflib.unified_diff(a, b, 'source', 'converted', n=2, lineterm=''))
    if not diff:
        print('MATCH: both sides agree on every event')
    else:
        print(f'DIFFER ({sum(1 for d in diff if d[:1] in "+-" and d[:3] not in ("+++", "---"))} lines):')
        print('\n'.join(diff[:200]))
        print('text differences (source -> converted, count):')
        for (a, b), n in text_diffs(diff):
            print(f'  {n:4}  {a!r} -> {b!r}')
    return 0 if not diff else 1


if __name__ == '__main__':
    sys.exit(main())
