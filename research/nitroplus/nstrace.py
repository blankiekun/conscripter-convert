"""Route tracer for a converted 装甲悪鬼村正 0.txt (docs/new-game.md step 11.3-11.4).

Runs the control flow of the NScripter script that `convert-nss` writes
(labels, goto, if, mov/add/sub/mul/div, select, end; everything else is
skipped) along a choice path, and prints the story files it passes, the
choices it takes and the ending it reaches. Variables from %800 on are
NScripter globals (the game's `#` flags): they survive NEWGAME, the rest
is cleared, as when a new game starts from the title.

    python3 -I research/nitroplus/nstrace.py <0.txt> <path file>...
        [--route out/muramasa-route.txt] [--quiet]
        [--flags <flag table> --watch <name>,<name>...]

A path file has one entry per line (`#` starts a comment):
    ma02_003:1        at the next select, which must be in ma02_003, take
                      option 1
    "信用できる"       take the option with this text (wherever it is)
    *                 take option 1 of whatever select comes next
    NEWGAME           start again at *start, keeping the globals
    SAVE name / LOAD name
                      remember the run's state / go back to it
    PUZZLE 1 / PUZZLE 0
                      at the next tile puzzle (the 立方陣), solve it / fail
    END file          the run must have ended in `file` (an ending file)
    REACH file        the run must have passed through `file` by now
Several path files run one after another in the same session (globals
carry over), like playing the routes in order. Each file after the first
starts a new game from the title, unless it starts with LOAD.

With --route, every step from one story file to the next is looked up in
the source route map (`conscripter-convert nss route` output): a step the
source can't take, or a choice option that leads elsewhere in the
source, is reported as a mismatch. That is the source side of the check.

--flags reads the flag table that `NSS_TRACE=1 conscripter-convert
convert-nss` prints on stderr (`flag %308 $Ichizyou_Flag`); --watch then
prints the named flags (`$Ichizyou_Flag`, or a part of a name such as
`死亡`) at each story file where one of them changed.
"""
import re
import sys
from collections import defaultdict

GLOBAL_BORDER = 800
# A path that went wrong usually loops (retry screens): stop.
MAX_PROBLEMS = 20


def load_script(path):
    lines = open(path, 'rb').read().decode('cp932').split('\n')
    lines = [l.rstrip('\r') for l in lines]
    labels = {l[1:].strip(): i for i, l in enumerate(lines) if l.startswith('*')}
    return lines, labels


def split_cmds(line):
    """Splits a line at the colons outside strings."""
    out, cur, quoted = [], '', False
    for c in line:
        if c == '"':
            quoted = not quoted
        if c == ':' and not quoted:
            out.append(cur)
            cur = ''
        else:
            cur += c
    out.append(cur)
    return [c.strip() for c in out if c.strip()]


def load_route(path):
    """{file: [(next file, condition text)]} from the route map."""
    edges = defaultdict(list)
    cur = None
    for line in open(path, encoding='utf-8'):
        m = re.match(r'(\S+\.nss)\s*$', line)
        if m:
            cur = m.group(1)
            continue
        m = re.match(r'\s+->\s+(\S+)\s*(?:\[(.*)\])?', line)
        if m and cur:
            edges[cur].append((m.group(1), m.group(2) or ''))
    return edges


def normalize(text):
    # The backend makes half-width characters full-width in vertical text.
    table = {chr(c): chr(c + 0xFEE0) for c in range(0x21, 0x7F)}
    return ''.join(table.get(c, c) for c in text).replace(' ', '').replace('　', '')


FULL_DIGITS = str.maketrans('１２３４５６７８９０', '1234567890')


class Run:
    def __init__(self, lines, labels, route, quiet, watch=None):
        self.lines, self.labels, self.route, self.quiet = lines, labels, route, quiet
        self.watch = watch or {}  # {variable number: flag name}
        self.watched = None
        self.var = defaultdict(int)
        self.svar = defaultdict(str)
        self.saves = {}
        self.problems = 0
        self.new_game()

    def new_game(self):
        for k in [k for k in self.var if k < GLOBAL_BORDER]:
            del self.var[k]
        self.svar.clear()
        self.pc = self.labels['start'] + 1
        self.file = None
        self.files = []
        self.pages = 0
        self.ended = None
        self.reported = False
        self.last_choice = None  # (file, option) of the select that ended the current file

    def problem(self, text):
        self.problems += 1
        print(f'  !! {text}')
        if self.problems >= MAX_PROBLEMS:
            self.report()
            print(f'stopping after {MAX_PROBLEMS} problems')
            sys.exit(1)

    def val(self, t):
        t = t.strip()
        if re.fullmatch(r'-?\d+', t):
            return int(t)
        if re.fullmatch(r'%\d+', t):
            return self.var[int(t[1:])]
        e = re.sub(r'%(\d+)', lambda m: str(self.var[int(m.group(1))]), t)
        if re.fullmatch(r'[-+*/() \d]+', e):
            return int(eval(e))
        raise ValueError(f'value {t!r}')

    def cond(self, c):
        for part in c.split('&&'):
            m = re.match(r'\s*(\S+)\s*(==|!=|>=|<=|>|<)\s*(\S+)\s*$', part)
            a, op, b = self.val(m.group(1)), m.group(2), self.val(m.group(3))
            if not {'==': a == b, '!=': a != b, '>=': a >= b, '<=': a <= b, '>': a > b, '<': a < b}[op]:
                return False
        return True

    def enter(self, label):
        """Notes a story file start (labels *f_<file>)."""
        if not label.startswith('f_'):
            return
        name = label[2:] + '.nss'
        if self.route is not None and self.file is not None:
            nexts = self.route.get(self.file, [])
            if not any(n == name for n, _ in nexts):
                self.problem(f'{self.file} -> {name}: not an edge of the source route map '
                             f'(source: {", ".join(n for n, _ in nexts) or "none"})')
            elif self.last_choice and self.last_choice[0] == self.file:
                # The source labels choice edges with the case label, which
                # ends in the option number (@選択肢１).
                opt = self.last_choice[1]
                want = [n for n, c in nexts if re.search(rf'choice @\D*{opt}\]?$', c.translate(FULL_DIGITS))]
                # An edge several options take is merged in the route map
                # and loses its option label: any option may take it.
                labelled = any('choice @' in c for n, c in nexts if n == name)
                if want and name not in want and labelled:
                    self.problem(f'{self.file} option {opt} -> {name}, the source goes to {", ".join(want)}')
        self.file = name
        self.files.append([name, []])
        self.last_choice = None
        if self.watch:
            now = {n: self.var[n] for n in self.watch}
            if now != self.watched:
                print(f'  {name}: ' + ' '.join(f'{self.watch[n]}={v}' for n, v in now.items()))
                self.watched = now

    def jump(self, target):
        label = target.strip().lstrip('*')
        if label not in self.labels:
            self.problem(f'missing label *{label}')
            return len(self.lines)
        self.enter(label)
        return self.labels[label] + 1

    def run_cmd(self, cmd):
        """Returns the new pc, or None to go on."""
        m = re.match(r'if (.+?) ((?:goto|mov|add|sub|mul|div|select|end|return)\b.*)$', cmd)
        if m:
            if not self.cond(m.group(1)):
                return 'skip'
            cmd = m.group(2)
        op, _, rest = cmd.partition(' ')
        if op == 'goto':
            return self.jump(rest)
        if op in ('mov', 'add', 'sub', 'mul', 'div'):
            a, b = rest.split(',', 1)
            a = a.strip()
            if a.startswith('$'):
                self.svar[a] = b.strip().strip('"')
                return None
            i = int(a[1:])
            v = self.val(b)
            self.var[i] = {'mov': v, 'add': self.var[i] + v, 'sub': self.var[i] - v,
                           'mul': self.var[i] * v, 'div': int(self.var[i] / v) if v else 0}[op]
            return None
        if op == 'select':
            items = re.findall(r'"((?:[^"])*)"\s*,\s*\*(\w+)', rest)
            return self.choose(items)
        if op == 'end':
            return len(self.lines)
        return None

    def choose(self, items):
        texts = [t for t, _ in items]
        entry = self.path[self.at] if self.at < len(self.path) else None
        pick = None
        if entry is None or entry[0] in ('NEWGAME', 'SAVE', 'LOAD', 'END', 'REACH'):
            self.problem(f'{self.file}: a select ({" / ".join(texts)}) the path has no entry for; taking 1')
            pick = 1
        else:
            kind, arg = entry
            self.at += 1
            if kind == 'any':
                pick = 1
            elif kind == 'opt':
                want_file, n = arg
                if want_file + '.nss' != self.file:
                    self.problem(f'path expects a select in {want_file}, this one is in {self.file} '
                                 f'({" / ".join(texts)})')
                pick = n
            else:
                hits = [i + 1 for i, t in enumerate(texts) if normalize(t) == normalize(arg)]
                if not hits:
                    self.problem(f'{self.file}: no option "{arg}" in ({" / ".join(texts)}); taking 1')
                pick = hits[0] if hits else 1
        if not 1 <= pick <= len(items):
            self.problem(f'{self.file}: option {pick} of {len(items)}; taking 1')
            pick = 1
        self.files[-1][1].append((pick, texts[pick - 1], len(texts)))
        self.last_choice = (self.file, pick)
        return self.jump(items[pick - 1][1])

    def puzzle(self, pc):
        """A tile puzzle (`; tile puzzle` ... its `*conv_puzzleN_answer`
        block): sets its result as the path says and returns the pc after
        the puzzle. Its own loop isn't run."""
        entry = self.path[self.at] if self.at < len(self.path) else None
        if entry is None or entry[0] != 'PUZZLE':
            self.problem(f'{self.file}: a tile puzzle the path has no PUZZLE entry for; solving it')
            solved = 1
        else:
            self.at += 1
            solved = entry[1]
        answer = next(i for i in range(pc, len(self.lines)) if re.fullmatch(r'\*conv_puzzle\d+_answer', self.lines[i]))
        result = next(i for i in range(answer, len(self.lines)) if re.fullmatch(r'mov %\d+,%191', self.lines[i]))
        self.var[int(self.lines[result].split(',')[0][5:])] = solved
        black = next(i for i in range(result, len(self.lines)) if self.lines[i] == 'bg "#000000",0')
        self.files[-1][1].append((solved, 'solved' if solved else 'failed', 2))
        return black + 2

    def run_until_stop(self, limit=5_000_000):
        """Runs until the script ends or the next path entry isn't a choice."""
        steps = 0
        while self.pc < len(self.lines):
            if self.at < len(self.path) and self.path[self.at][0] in ('NEWGAME', 'SAVE', 'LOAD', 'END', 'REACH'):
                if self.path[self.at][0] in ('SAVE', 'REACH'):
                    return
            steps += 1
            if steps > limit:
                self.problem(f'{limit} steps in {self.file}: a loop?')
                self.pc = len(self.lines)
                break
            line = self.lines[self.pc]
            if line.startswith('*'):
                self.enter(line[1:].strip())
                self.pc += 1
                continue
            if line == '; tile puzzle':
                self.pc = self.puzzle(self.pc)
                continue
            if not line or line.startswith(';') or not re.match(r'[a-z]', line):
                if line.endswith('\\'):
                    self.pages += 1
                self.pc += 1
                continue
            new = None
            for cmd in split_cmds(line):
                r = self.run_cmd(cmd)
                if r == 'skip':
                    break  # `if` false: the rest of the line belongs to it
                if r is not None:
                    new = r
                    break
            self.pc = self.pc + 1 if new is None else new
        self.ended = self.file

    def play(self, path):
        self.path, self.at = path, 0
        while True:
            self.run_until_stop()
            if self.at >= len(self.path):
                if self.ended is not None and not self.reported:
                    self.report()
                return
            kind, arg = self.path[self.at]
            self.at += 1
            if kind == 'SAVE':
                self.saves[arg] = (dict(self.var), dict(self.svar), self.pc, self.file,
                                   [[f, list(c)] for f, c in self.files], self.last_choice)
                print(f'  SAVE {arg} in {self.file}')
            elif kind == 'REACH':
                if not any(f == arg + '.nss' for f, _ in self.files):
                    self.problem(f'expected to pass {arg} by now (now in {self.file})')
            elif self.ended is None:
                self.problem(f'{kind} while the run is still going (in {self.file})')
                return
            elif kind == 'END':
                self.report()
                if self.ended != arg + '.nss':
                    self.problem(f'expected to end in {arg}, ended in {self.ended}')
                else:
                    print(f'  ok: ended in {arg}')
            elif kind == 'NEWGAME':
                print('NEWGAME (globals kept: ' + ', '.join(f'%{k}={v}' for k, v in sorted(self.var.items()) if k >= GLOBAL_BORDER and v) + ')')
                self.new_game()
            elif kind == 'LOAD':
                var, svar, pc, file, files, last = self.saves[arg]
                globals_now = {k: v for k, v in self.var.items() if k >= GLOBAL_BORDER}
                self.var = defaultdict(int, {k: v for k, v in var.items() if k < GLOBAL_BORDER})
                self.var.update(globals_now)
                self.svar = defaultdict(str, svar)
                self.pc, self.file, self.files, self.last_choice = pc, file, files, last
                self.ended = None
                self.reported = False
                print(f'  LOAD {arg} in {file}')
            else:
                self.problem(f'path entry {arg} left over: the run ended in {self.ended}')

    def report(self):
        self.reported = True
        if self.quiet:
            print(f'  {len(self.files)} files, {self.pages} pages, ended in {self.ended}')
            return
        for name, choices in self.files:
            picks = ' '.join(f'[{n}/{k} {t}]' for n, t, k in choices)
            print(f'  {name} {picks}'.rstrip())
        print(f'  -> {len(self.files)} files, {self.pages} pages, ended in {self.ended}')


def read_path(path):
    entries = []
    for raw in open(path, encoding='utf-8'):
        line = raw.split('#', 1)[0].strip()
        if not line:
            continue
        word, _, arg = line.partition(' ')
        if word in ('NEWGAME',):
            entries.append((word, None))
        elif word == 'PUZZLE':
            entries.append((word, int(arg)))
        elif word in ('SAVE', 'LOAD', 'END', 'REACH'):
            entries.append((word, arg.strip()))
        elif line == '*':
            entries.append(('any', None))
        elif line.startswith('"'):
            entries.append(('text', line.strip('"')))
        else:
            f, _, n = line.rpartition(':')
            entries.append(('opt', (f.removesuffix('.nss'), int(n))))
    return entries


def main():
    args = sys.argv[1:]
    route = None
    if '--route' in args:
        i = args.index('--route')
        route = load_route(args[i + 1])
        del args[i:i + 2]
    watch = {}
    if '--flags' in args:
        i = args.index('--flags')
        table = {}
        for line in open(args[i + 1], encoding='utf-8'):
            m = re.match(r'flag %(\d+) (.+)$', line.strip())
            if m:
                table[m.group(2)] = int(m.group(1))
        del args[i:i + 2]
        if '--watch' in args:
            i = args.index('--watch')
            for want in args[i + 1].split(','):
                hits = [k for k in table if k == want] or [k for k in table if want in k]
                if not hits:
                    sys.exit(f'no flag {want} in the table')
                for k in hits:
                    watch[table[k]] = k
            del args[i:i + 2]
    quiet = '--quiet' in args
    args = [a for a in args if a != '--quiet']
    lines, labels = load_script(args[0])
    run = Run(lines, labels, route, quiet, watch)
    for p in args[1:]:
        print(f'== {p}')
        path = read_path(p)
        # A path file is a new playthrough from the title, unless it
        # starts by loading a save.
        if run.ended is not None and path and path[0][0] not in ('LOAD', 'NEWGAME'):
            path = [('NEWGAME', None)] + path
        run.play(path)
    print(f'{run.problems} problem(s)')
    sys.exit(1 if run.problems else 0)


if __name__ == '__main__':
    main()
