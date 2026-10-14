"""CRI CPK archives: @UTF tables, TOC listing, CRILAYLA decompression.

    python3 -I cpk.py <file.cpk>                  list the TOC
    python3 -I cpk.py <file.cpk> <out> [glob]     extract (decompressing CRILAYLA)
"""
import fnmatch
import os
import struct
import sys

# column type -> (struct code, size); 0xA string, 0xB data (offset, size)
TYPES = {0: ">B", 1: ">b", 2: ">H", 3: ">h", 4: ">I", 5: ">i", 6: ">Q", 7: ">q",
         8: ">f", 9: ">d", 0xA: ">I", 0xB: ">II"}


def utf_table(buf):
    """Parse an @UTF table into (name, [row dicts])."""
    if buf[:4] != b"@UTF":
        raise ValueError("not an @UTF table: %r" % buf[:4])
    size, rows_off, str_off, data_off, name_off, ncols, row_len, nrows = \
        struct.unpack(">IIIIIHHI", buf[4:32])
    base = 8

    def cstr(off):
        s = base + str_off + off
        return buf[s:buf.index(b"\0", s)].decode("cp932", "replace")

    def value(t, pos):
        code = TYPES[t]
        v = struct.unpack_from(code, buf, pos)
        n = struct.calcsize(code)
        if t == 0xA:
            return cstr(v[0]), n
        if t == 0xB:
            s = base + data_off + v[0]
            return bytes(buf[s:s + v[1]]), n
        return v[0], n

    cols = []
    pos = 32
    for _ in range(ncols):
        flags = buf[pos]
        name = cstr(struct.unpack_from(">I", buf, pos + 1)[0])
        pos += 5
        storage, t = flags & 0xF0, flags & 0x0F
        const = None
        if storage == 0x30:
            const, n = value(t, pos)
            pos += n
        cols.append((name, storage, t, const))
    rows = []
    for r in range(nrows):
        pos = base + rows_off + r * row_len
        row = {}
        for name, storage, t, const in cols:
            if storage == 0x50:
                row[name], n = value(t, pos)
                pos += n
            elif storage == 0x30:
                row[name] = const
            else:
                row[name] = None
        rows.append(row)
    return cstr(name_off), rows


def chunk_table(f, off):
    """A CPK chunk ('CPK ', 'TOC ', ...): 16-byte header, then @UTF."""
    f.seek(off)
    head = f.read(16)
    size = struct.unpack_from("<Q", head, 8)[0]
    return head[:4], utf_table(f.read(size))


def crilayla(src):
    usize, hoff = struct.unpack_from("<II", src, 8)
    out = bytearray(usize + 0x100)
    out[:0x100] = src[0x10 + hoff:0x10 + hoff + 0x100]
    pos = 0x10 + hoff - 1
    pool = 0
    left = 0

    def bits(n):
        nonlocal pos, pool, left
        v = 0
        while n:
            if left == 0:
                pool = src[pos]
                pos -= 1
                left = 8
            take = min(left, n)
            v = (v << take) | ((pool >> (left - take)) & ((1 << take) - 1))
            left -= take
            n -= take
        return v

    end = 0x100 + usize - 1
    done = 0
    while done < usize:
        if bits(1):
            ref = end - done + bits(13) + 3
            length = 3
            for w in (2, 3, 5, 8):
                v = bits(w)
                length += v
                if v != (1 << w) - 1:
                    break
            else:
                while True:
                    v = bits(8)
                    length += v
                    if v != 255:
                        break
            for _ in range(length):
                out[end - done] = out[ref]
                ref -= 1
                done += 1
        else:
            out[end - done] = bits(8)
            done += 1
    return bytes(out)


def toc(path):
    with open(path, "rb") as f:
        _, (_, hdr) = chunk_table(f, 0)
        h = hdr[0]
        if not h.get("TocOffset"):
            yield from itoc(f, h)
            return
        _, (_, files) = chunk_table(f, h["TocOffset"])
        add = h["TocOffset"]
        if h.get("ContentOffset") and h["ContentOffset"] < add:
            add = h["ContentOffset"]
        for e in files:
            name = (e["DirName"] + "/" if e.get("DirName") else "") + e["FileName"]
            yield name, add + e["FileOffset"], e["FileSize"], e["ExtractSize"], e


def itoc(f, h):
    """ID-only CPK (AWB-style): files in ID order from ContentOffset, aligned."""
    _, (_, rows) = chunk_table(f, h["ItocOffset"])
    entries = []
    for key in ("DataL", "DataH"):
        if rows[0].get(key):
            entries += utf_table(rows[0][key])[1]
    entries.sort(key=lambda e: e["ID"])
    align = h.get("Align") or 1
    off = h["ContentOffset"]
    for e in entries:
        yield "%05d" % e["ID"], off, e["FileSize"], e["ExtractSize"], e
        off += (e["FileSize"] + align - 1) // align * align


def afs2(path):
    """AFS2 (.awb): ids, then (count + 1) offsets; each start aligned up."""
    with open(path, "rb") as f:
        head = f.read(16)
        if head[:4] != b"AFS2":
            return None
        osize, isize = head[5], struct.unpack_from("<H", head, 6)[0]
        count, align = struct.unpack_from("<II", head, 8)
        align &= 0xFFFF
        ids = f.read(count * isize)
        offs = f.read((count + 1) * osize)
    icode = {2: "<H", 4: "<I"}[isize]
    ocode = {2: "<H", 4: "<I"}[osize]
    out = []
    for i in range(count):
        a = struct.unpack_from(ocode, offs, i * osize)[0]
        b = struct.unpack_from(ocode, offs, (i + 1) * osize)[0]
        a = (a + align - 1) // align * align
        fid = struct.unpack_from(icode, ids, i * isize)[0]
        out.append(("%05d" % fid, a, b - a, b - a, None))
    return out


def read(f, off, size, extract):
    f.seek(off)
    data = f.read(size)
    if data[:8] == b"CRILAYLA":
        data = crilayla(data)
    return data


def main():
    path = sys.argv[1]
    entries = afs2(path)
    if entries is None:
        entries = list(toc(path))
    if len(sys.argv) == 2 and entries and entries[0][4] is None:
        for name, off, size, ext, _ in entries:
            print("%10d %10d %10d %s" % (off, size, ext, name))
        return
    if len(sys.argv) == 2:
        with open(path, "rb") as f:
            print(chunk_table(f, 0)[1][1][0])
        for name, off, size, ext, _ in entries:
            print("%10d %10d %10d %s" % (off, size, ext, name))
        return
    out = sys.argv[2]
    pat = sys.argv[3] if len(sys.argv) > 3 else "*"
    with open(path, "rb") as f:
        for name, off, size, ext, _ in entries:
            if not fnmatch.fnmatch(name, pat):
                continue
            dst = os.path.join(out, name)
            os.makedirs(os.path.dirname(dst) or ".", exist_ok=True)
            with open(dst, "wb") as o:
                o.write(read(f, off, size, ext))


if __name__ == "__main__":
    main()
