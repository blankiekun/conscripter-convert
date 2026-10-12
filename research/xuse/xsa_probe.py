import struct, sys, os, collections
for path in sys.argv[1:]:
    b = open(path,'rb').read()
    ver, = struct.unpack_from('<I', b, 0)
    first_off = struct.unpack_from('<I', b, 8)[0]
    nrec = (first_off - 4) // 16
    recs = [struct.unpack_from('<4I', b, 4+16*i) for i in range(nrec)]
    used = [(i,r) for i,r in enumerate(recs) if r[0] or r[1]]
    chained = all(used[k][1][1]+used[k][1][0] == used[k+1][1][1] for k in range(len(used)-1))
    last = used[-1][1]
    extra = collections.Counter((r[2], r[3]) for _,r in used)
    magics = collections.Counter(b[r[1]:r[1]+4] for _,r in used)
    print(os.path.basename(path), f'ver={ver} first_off={first_off:#x} slots={nrec} used={len(used)} '
          f'chained={chained} end={last[1]+last[0]} filesize={len(b)} extra={dict(extra) if len(extra)<4 else len(extra)} '
          f'magics={dict(magics.most_common(3))} unused_idx_sample={[i for i,r in enumerate(recs) if not (r[0] or r[1])][:8]}')
