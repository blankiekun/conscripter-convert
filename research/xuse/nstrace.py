"""Tiny interpreter for the NScripter subset vn2ons emits; traces control flow.
usage: python3 -I nstrace.py 0.txt [max_steps] [--show] [--pages N]
--pages N: keep going past text pages; print the script-file sequence and
the line of every 20th displayed page, stop after N pages.
--click K: at a link page (btnwait), click link K (1-based) the first time
that page is shown; otherwise, and by default, click past it."""
import sys, re, collections
lines = open(sys.argv[1], 'rb').read().decode('cp932').split('\n')
limit = int(sys.argv[2]) if len(sys.argv) > 2 else 200000
labels = {l[1:].strip(): i for i, l in enumerate(lines) if l.startswith('*')}
var = collections.defaultdict(int); stack = []; hits = collections.Counter()
def val(t):
    t = t.strip()
    return var[int(t[1:])] if t.startswith('%') else int(t)
def cond(c):
    for part in c.split('&&'):
        m = re.match(r'\s*(\S+)\s*(==|!=|>=|<=|>|<)\s*(\S+)\s*$', part)
        a, op, b = val(m.group(1)), m.group(2), val(m.group(3))
        if not {'==': a == b, '!=': a != b, '>=': a >= b, '<=': a <= b, '>': a > b, '<': a < b}[op]: return False
    return True
pc = labels['start'] + 1; steps = 0; pages = 0
maxpages = int(sys.argv[sys.argv.index('--pages') + 1]) if '--pages' in sys.argv else 0
cur_script = None
click = int(sys.argv[sys.argv.index('--click') + 1]) if '--click' in sys.argv else 0
clicked = set()
def run_cmd(cmd):
    """returns new pc or None to continue"""
    global pc, pages
    cmd = cmd.strip()
    m = re.match(r'if (.+?) (goto|gosub|return|mov|add|sub|mul|div|mod|bg)\b(.*)$', cmd)
    if m:
        return run_cmd(m.group(2) + m.group(3)) if cond(m.group(1)) else None
    w = cmd.split(' ', 1); op = w[0]; rest = w[1] if len(w) > 1 else ''
    if op == 'goto': return labels[rest.strip()[1:]]
    if op == 'gosub': stack.append(pc + 1); return labels[rest.strip()[1:]]
    if op == 'return': return stack.pop() if stack else len(lines)
    if op in ('mov', 'add', 'sub', 'mul', 'div', 'mod'):
        a, b = rest.split(',', 1); i = int(a.strip()[1:]); v = val(b)
        var[i] = {'mov': v, 'add': var[i] + v, 'sub': var[i] - v, 'mul': var[i] * v,
                  'div': int(var[i] / v) if v else 0, 'mod': var[i] % v if v else 0}[op]
    if op == 'btnwait':
        pages += 1
        choice = click if click and pc not in clicked else 0
        clicked.add(pc); var[int(rest.strip()[1:])] = choice
        if maxpages and choice: print(f"  click link {choice} at line {pc} (page {pages})")
    if op == 'end': return len(lines)
    return None
while pc < len(lines) and steps < limit:
    l = lines[pc]; steps += 1; hits[pc] += 1
    if l.endswith('\\') and not re.match(r'^[a-z;*]', l):
        pages += 1
        if not maxpages:
            print(f"first text page at line {pc} after {steps} steps"); break
        if pages == 1 or pages % 20 == 0: print(f"  page {pages} at line {pc}: {l[:60]}")
        if pages >= maxpages: print(f"stopped after {pages} pages, {steps} steps"); break
    if l.startswith('*'):
        m = re.match(r'\*s(\d+)_', l)
        if m and m.group(1) != cur_script and maxpages:
            cur_script = m.group(1); print(f"-> script {cur_script} (line {pc}, page {pages})")
    if not l or l.startswith((";", "*")): pc += 1; continue
    if "--show" in sys.argv: print(f"  {pc:6} {l[:100]}")
    new = None
    for c in l.split(':'):
        new = run_cmd(c)
        if new is not None: break
    pc = new if new is not None else pc + 1
else:
    print(f"no text page within {steps} steps (pc={pc}); hottest lines:")
    for ln, n in hits.most_common(12): print(f"  {n:7}x line {ln}: {lines[ln][:110]}")
    print("call stack depth:", len(stack))
