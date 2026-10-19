#!/usr/bin/env python3
"""Dump an emulated game's memory through the emulator's GDB stub (gdb remote
protocol over TCP), in the dump format exe_call's load_dump and dumppack's
Memory read (ntrace.c NTRACE_DUMPS: <prefix>.map lines `base size protect type
offset`, all hex, and <prefix>.bin).

    python3 -I research/tools/gdbdump.py HOST:PORT PREFIX [--range START:LEN ...] [--scan LO:HI:STEP ...]
        [--get-info] [--chunk N]
    python3 -I research/tools/gdbdump.py --self-test

The regions come from the stub's memory map (qXfer:memory-map) when it gives
one, else from --range (hex start:length; for an emulator whose stub has no
map), or --scan (hex lo:hi:step: one byte read every step, the readable
stretches kept; Vita3K's stub, which has no map), or with --get-info from Ryujinx's `monitor get info` (the modules, and
the heap from its start as far as it reads; the text goes to <prefix>.info). Each region is read with `m` packets of --chunk bytes (default: what the
stub's PacketSize allows, at most 0x7000); a chunk the stub refuses (`E..`) is
left out, so a region may split. The registers of the stopped thread (`g`) go
to <prefix>.regs as hex. The target stops while it is read (as gdb's attach
does); the dump ends with a detach (`D`), which lets it run on.
"""
import re
import socket
import sys


class Gdb:
    def __init__(self, host, port, timeout=30):
        self.s = socket.create_connection((host, port), timeout=timeout)
        # no Nagle: an ack and the next request are small writes, and waiting on the delayed ACK between them made each packet ~130 ms
        self.s.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        self.buf = b""
        self.ack = True

    def _read(self, n=1):
        while len(self.buf) < n:
            d = self.s.recv(65536)
            if not d:
                raise EOFError("stub closed the connection")
            self.buf += d
        out, self.buf = self.buf[:n], self.buf[n:]
        return out

    def send(self, data):
        pkt = b"$" + data + b"#" + b"%02x" % (sum(data) & 0xff)
        for _ in range(5):
            self.s.sendall(pkt)
            if not self.ack:
                break
            c = self._read()
            while c not in b"+-":  # stray bytes before the ack
                c = self._read()
            if c == b"+":
                break
        return self.recv()

    def recv(self):
        while True:
            c = self._read()
            if c != b"$":
                continue
            body = b""
            while True:
                c = self._read()
                if c == b"#":
                    break
                body += c
            self._read(2)
            if self.ack:
                self.s.sendall(b"+")
            body = unescape(body)
            if body.startswith(b"O") and len(body) > 1 and re.fullmatch(rb"O[0-9a-fA-F]+", body):
                continue  # the target's console output
            return body


def unescape(b):
    """gdb remote: `}` escapes (x ^ 0x20) and `*` run lengths"""
    out, i = bytearray(), 0
    while i < len(b):
        c = b[i]
        if c == 0x7d:
            out.append(b[i + 1] ^ 0x20)
            i += 2
        elif c == 0x2a and out:
            out += bytes([out[-1]]) * (b[i + 1] - 29)
            i += 2
        else:
            out.append(c)
            i += 1
    return bytes(out)


def memory_map(g):
    xml, off = b"", 0
    while True:
        r = g.send(b"qXfer:memory-map:read::%x,fff" % off)
        if not r or r[:1] not in b"ml":
            return []
        xml += r[1:]
        off += len(r) - 1
        if r[:1] == b"l":
            break
    return [(int(a, 16), int(n, 16)) for a, n in re.findall(rb'<memory[^>]*start="0x([0-9a-fA-F]+)"[^>]*length="0x([0-9a-fA-F]+)"', xml)]


def monitor(g, cmd):
    r = g.send(b"qRcmd," + cmd.encode().hex().encode())
    try:
        return bytes.fromhex(r.decode()).decode(errors="replace")
    except ValueError:
        return ""


def info_ranges(g, step=0x100000, cap=0x60000000):
    """Ryujinx's stub has no memory map, but `monitor get info` lists the
    modules and the address space's layout: the modules whole, and the heap
    from its start as far as it reads (probed a MB at a time, up to `cap`)"""
    info = monitor(g, "get info")
    out = [(int(a, 16), int(b, 16) + 1 - int(a, 16)) for a, b in re.findall(r"^\s+0x([0-9a-f]+) - 0x([0-9a-f]+) \S", info, re.M)]
    h = re.search(r"Heap:\s+0x([0-9a-f]+) - 0x([0-9a-f]+)", info)
    if h:
        lo, hi = int(h.group(1), 16), int(h.group(2), 16)
        end = lo
        while end < min(hi, lo + cap):
            r = g.send(b"m%x,10" % end)
            if not r or r[:1] == b"E":
                break
            end += step
        if end > lo:
            out.append((lo, end - lo))
    return out, info


def scan(g, lo, hi, step):
    """the readable stretches of [lo, hi), probed a byte every `step` (a stub
    with no memory map, Vita3K's: it refuses a read outside allocated memory)"""
    out, start = [], None
    for a in range(lo, hi, step):
        r = g.send(b"m%x,1" % a)
        ok = len(r) == 2 and r[:1] != b"E"
        if ok and start is None:
            start = a
        elif not ok and start is not None:
            out.append((start, a - start))
            start = None
    if start is not None:
        out.append((start, hi - start))
    return out


def dump(g, prefix, ranges, chunk):
    total, lines = 0, []
    with open(prefix + ".bin", "wb") as b:
        for start, length in ranges:
            run, at = bytearray(), start
            for a in range(start, start + length, chunk):
                n = min(chunk, start + length - a)
                r = g.send(b"m%x,%x" % (a, n))
                ok = r and not (len(r) == 3 and r[:1] == b"E") and re.fullmatch(rb"[0-9a-fA-F]*", r) and len(r) == 2 * n
                if ok:
                    if not run:
                        at = a
                    run += bytes.fromhex(r.decode())
                if (not ok or a + n >= start + length) and run:
                    lines.append(f"{at:08x} {len(run):08x} 4 20000 {total:x}\n")
                    b.write(run)
                    total += len(run)
                    run = bytearray()
    open(prefix + ".map", "w").write("".join(lines))
    return total, len(lines)


def main(argv):
    if argv == ["--self-test"]:
        return self_test()
    if len(argv) < 2:
        sys.exit(__doc__)
    host, port = argv[0].rsplit(":", 1)
    prefix = argv[1]
    ranges = [tuple(int(x, 16) for x in v.split(":")) for k, v in zip(argv, argv[1:]) if k == "--range"]
    g = Gdb(host, int(port))
    sup = g.send(b"qSupported:multiprocess+;xmlRegisters=i386")
    if b"QStartNoAckMode+" in sup and g.send(b"QStartNoAckMode") == b"OK":
        g.ack = False
    m = re.search(rb"PacketSize=([0-9a-fA-F]+)", sup)
    chunk = int(argv[argv.index("--chunk") + 1], 0) if "--chunk" in argv else min(0x7000, (int(m.group(1), 16) - 16) // 2 if m else 0x400)
    stop = g.send(b"?")
    info = ""
    if not ranges and "--get-info" in argv:
        ranges, info = info_ranges(g)
        open(prefix + ".info", "w").write(info)
    for k, v in zip(argv, argv[1:]):
        if k == "--scan" and not ranges:
            lo, hi, step = (int(x, 16) for x in v.split(":"))
            ranges += scan(g, lo, hi, step)
    ranges = ranges or memory_map(g)
    if not ranges:
        sys.exit("the stub gives no memory map: name the regions with --range START:LEN")
    total, n = dump(g, prefix, ranges, chunk)
    open(prefix + ".regs", "w").write(g.send(b"g").decode(errors="replace") + "\n")
    print(f"{prefix}: {n} regions, {total} bytes (stop reply {stop[:20]!r}, chunk {chunk:#x})")
    try:  # detach: the target runs on (Ryujinx's stub doesn't answer it: don't wait)
        g.s.sendall(b"$D#44")
        g.s.close()
    except OSError:
        pass


def self_test():
    """a fake stub over a socket: memory map, a refused chunk, the dump read back"""
    import os
    import tempfile
    import threading
    mem = {0x1000 + i: (i * 7) & 0xff for i in range(0x3000)}
    srv = socket.socket()
    srv.bind(("127.0.0.1", 0))
    srv.listen(1)

    def serve():
        c, _ = srv.accept()
        f = Gdb.__new__(Gdb)
        f.s, f.buf, f.ack = c, b"", True

        def reply(data):
            c.sendall(b"$" + data + b"#" + b"%02x" % (sum(data) & 0xff))
        while True:
            try:
                p = f.recv()
            except Exception:  # the client went: done
                return
            if p.startswith(b"qSupported"):
                reply(b"PacketSize=1000;qXfer:memory-map:read+")
            elif p == b"?":
                reply(b"S05")
            elif p.startswith(b"qXfer:memory-map"):
                reply(b'l<memory-map><memory type="ram" start="0x1000" length="0x3000"/></memory-map>')
            elif p.startswith(b"m"):
                a, n = (int(x, 16) for x in p[1:].split(b","))
                if 0x2000 <= a < 0x2800 or not 0x1000 <= a < 0x4000:
                    reply(b"E14")  # a page the stub can't read
                else:
                    reply(bytes(mem[a + i] for i in range(n)).hex().encode())
            elif p == b"g":
                reply(b"00" * 8)
            else:
                try:
                    reply(b"")
                except OSError:
                    return
    def serve_all():
        while True:
            serve()
    threading.Thread(target=serve_all, daemon=True).start()
    with tempfile.TemporaryDirectory() as d:
        pre = os.path.join(d, "x")
        main([f"127.0.0.1:{srv.getsockname()[1]}", pre, "--chunk", "0x400"])
        sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
        from dumppack import Memory
        m = Memory(pre)
        assert m.read(0x1000, 4) == bytes(mem[0x1000 + i] for i in range(4))
        assert m.read(0x3ffc, 4) == bytes(mem[0x3ffc + i] for i in range(4))
        assert m.read(0x2000, 2) == b"\0\0"  # left out
        assert len(open(pre + ".map").read().splitlines()) == 2
        main([f"127.0.0.1:{srv.getsockname()[1]}", pre + "s", "--scan", "0:8000:400", "--chunk", "0x400"])
        lines = open(pre + "s.map").read().split()
        assert lines[0] == "00001000" and int(lines[1], 16) == 0x1000, lines
    print("gdbdump self-test: ok")


if __name__ == "__main__":
    main(sys.argv[1:])
