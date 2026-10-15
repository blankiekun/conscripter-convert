"""Disassemble a compiled Squirrel 3 script from Full Metal Daemon Muramasa's
English release (script.npk, see npk.py).

    python3 -I nut.py FILE.nut            # every function: literals, code
    python3 -I nut.py FILE.nut --strings  # string literals only, in order
    python3 -I nut.py FILE.nut --tables   # a lang/en file's text tables

File: "SCRP", u32 header size (0x10), u32, u32; then a Squirrel 3 closure
stream: u16 0xFAFA, "RIQS", u32 sizeof(SQChar) = 1 (the stock sizes of
SQInteger and SQFloat are left out), the main function, "LIAT".
SQInteger is 32-bit; a function ends with u32 stack size, u8 generator,
u8 varparams. Opcodes are Squirrel 3.1's.
"""
import struct
import sys

OPS = ('LINE LOAD LOADINT LOADFLOAT DLOAD TAILCALL CALL PREPCALL PREPCALLK '
       'GETK MOVE NEWSLOT DELETE SET GET EQ NE ADD SUB MUL DIV MOD BITW '
       'RETURN LOADNULLS LOADROOT LOADBOOL DMOVE JMP JCMP JZ SETOUTER '
       'GETOUTER NEWOBJ APPENDARRAY COMPARITH INC INCL PINC PINCL CMP '
       'EXISTS INSTANCEOF AND OR NEG NOT BWNOT CLOSURE YIELD RESUME FOREACH '
       'POSTFOREACH CLONE TYPEOF PUSHTRAP POPTRAP THROW NEWSLOTA GETBASE '
       'CLOSE').split()


class Reader:
    def __init__(self, data, pos=0):
        self.d, self.p = data, pos

    def u32(self):
        v, = struct.unpack_from('<I', self.d, self.p)
        self.p += 4
        return v

    def tag(self, t):
        got = self.d[self.p:self.p + 4]
        assert got == t, (got, t, hex(self.p))
        self.p += 4

    def obj(self):
        t = self.u32()
        if t == 0x08000010:
            n = self.u32()
            s = self.d[self.p:self.p + n].decode('utf-8', 'replace')
            self.p += n
            return s
        if t == 0x05000002:
            return struct.unpack('<i', struct.pack('<I', self.u32()))[0]
        if t == 0x05000004:
            return struct.unpack('<f', struct.pack('<I', self.u32()))[0]
        if t == 0x01000008:
            return bool(self.u32())
        if t == 0x01000001:
            return None
        raise ValueError('object type %#x at %#x' % (t, self.p - 4))

    def function(self):
        self.tag(b'TRAP')
        f = {'source': self.obj(), 'name': self.obj()}
        self.tag(b'TRAP')
        (nlit, npar, nout, nloc, nline, ndef, nins, nfun) = (
            self.u32() for _ in range(8))
        self.tag(b'TRAP')
        f['literals'] = [self.obj() for _ in range(nlit)]
        self.tag(b'TRAP')
        f['params'] = [self.obj() for _ in range(npar)]
        self.tag(b'TRAP')
        f['outers'] = [(self.u32(), self.obj(), self.obj()) for _ in range(nout)]
        self.tag(b'TRAP')
        f['locals'] = [(self.obj(), self.u32(), self.u32(), self.u32())
                       for _ in range(nloc)]
        self.tag(b'TRAP')
        f['lines'] = [(self.u32(), self.u32()) for _ in range(nline)]
        self.tag(b'TRAP')
        f['defaults'] = [self.u32() for _ in range(ndef)]
        self.tag(b'TRAP')
        f['code'] = []
        for _ in range(nins):
            arg1, op, a0, a2, a3 = struct.unpack_from('<iBBBB', self.d, self.p)
            self.p += 8
            f['code'].append((op, a0, arg1, a2, a3))
        self.tag(b'TRAP')
        f['functions'] = [self.function() for _ in range(nfun)]
        f['stack'] = self.u32()
        f['generator'], f['varparams'] = self.d[self.p], self.d[self.p + 1]
        self.p += 2
        return f


def load(path):
    d = open(path, 'rb').read()
    assert d[:4] == b'SCRP', d[:4]
    r = Reader(d, struct.unpack_from('<I', d, 4)[0])
    assert d[r.p:r.p + 2] == b'\xfa\xfa'
    r.p += 2
    r.tag(b'RIQS')
    assert r.u32() == 1
    main = r.function()
    r.tag(b'LIAT')
    return main


def dump(f, depth=0):
    ind = '  ' * depth
    print('%sfunction %s (%s) params %s' % (ind, f['name'], f['source'], f['params']))
    lit = f['literals']
    for i, (op, a0, a1, a2, a3) in enumerate(f['code']):
        name = OPS[op] if op < len(OPS) else 'OP%d' % op
        note = ''
        if name in ('LOAD', 'PREPCALLK', 'GETK', 'DLOAD') and 0 <= a1 < len(lit):
            note = '  ; ' + repr(lit[a1])
        if name == 'DLOAD' and a3 < len(lit):
            note += ' ' + repr(lit[a3])
        print('%s  %4d %-11s %3d %6d %3d %3d%s' % (ind, i, name, a0, a1, a2, a3, note))
    for g in f['functions']:
        dump(g, depth + 1)


def tables(f):
    """A lang/en file's tables: {'TransText': {10: text, ...}, ...}, read from
    its `GETK table; LOADINT key; LOAD text; NEWSLOT` runs."""
    out, lit, code = {}, f['literals'], f['code']
    for i in range(len(code) - 3):
        ops = [OPS[c[0]] for c in code[i:i + 4]]
        if ops == ['GETK', 'LOADINT', 'LOAD', 'NEWSLOT']:
            out.setdefault(lit[code[i][2]], {})[code[i + 1][2]] = lit[code[i + 2][2]]
    return out


def strings(f):
    for s in f['literals']:
        if isinstance(s, str):
            yield s
    for g in f['functions']:
        yield from strings(g)


if __name__ == '__main__':
    main = load(sys.argv[1])
    if '--tables' in sys.argv:
        for name, rows in tables(main).items():
            for key, text in rows.items():
                print('%s[%d] %r' % (name, key, text))
    elif '--strings' in sys.argv:
        for s in strings(main):
            print(repr(s))
    else:
        dump(main)
