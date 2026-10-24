"""webshots.py: CDP screenshots of the web run. <outdir> [--every s] [--port p] [--for s]: connects to web_run's
Chromium over CDP and saves a small JPEG of the page every N s, named
<sec>_pg<page>.jpg (page = last trace record's pg)."""
import sys, time, os, json
from playwright.sync_api import sync_playwright
out = sys.argv[1]; a = sys.argv[2:]
def opt(n, d):
    return float(a[a.index(n) + 1]) if n in a else d
every, port, dur = opt('--every', 20), int(opt('--port', 9341)), opt('--for', 2000)
os.makedirs(out, exist_ok=True)
with sync_playwright() as p:
    for _ in range(60):
        try:
            b = p.chromium.connect_over_cdp(f'http://127.0.0.1:{port}'); break
        except Exception:
            time.sleep(1)
    t0 = time.time()
    page = None
    while time.time() - t0 < dur:
        try:
            if page is None:
                pages = [pg for c in b.contexts for pg in c.pages if 'conscripter' in pg.url]
                page = pages[0] if pages else None
            if page:
                pg = page.evaluate("""() => { const l = self.g_trace_lines || []; for (let i = l.length - 1; i >= 0 && i > l.length - 400; i--) { const m = l[i].match(/"pg":(\\d+)/); if (m) return m[1]; } return '0'; }""")
                page.screenshot(path=f'{out}/{int(time.time()-t0):05d}_pg{pg}.jpg', type='jpeg', quality=70)
        except Exception as e:
            print('shot:', str(e)[:200], file=sys.stderr)
            if 'closed' in str(e) or 'Target' in str(e):
                break
        time.sleep(every)
