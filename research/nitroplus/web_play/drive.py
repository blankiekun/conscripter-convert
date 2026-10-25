"""drive.py <outdir> --port p STEP...: drives the page web_run.py opened (over CDP, as
webshots.py) through STEPs, in order:
  pg:N         wait until the trace's last page record is N or more (120 s at most)
  label:NAME   wait for a "label" record naming NAME (trace, 120 s at most)
  sleep:S      wait S seconds
  key:K        press key K (Playwright names: s, l, b, t, Escape, Enter, PageUp, ...)
  click:X,Y    left click at game pixel X,Y (the canvas scaled to the game's size)
  rclick:X,Y   right click there
  wheel:N      N mouse wheel notches (negative: up)
  move:X,Y     move the pointer there
  adv:N        click the middle of the screen every ADV_MS (700) ms until page N (600 s at most)
  shot:NAME    save the canvas as <outdir>/NAME.png at the game's size, and print the page
  reload       reload the page (the save folder, IndexedDB, stays)
Each step prints one line: the step, seconds, last page. The viewport is set to the game's
size, so the canvas is drawn 1:1 when the page lays it out so."""
import sys, time, os, re
from playwright.sync_api import sync_playwright

a = sys.argv[1:]
out = a.pop(0)
port = int(a[a.index('--port') + 1]); i = a.index('--port'); del a[i:i + 2]
W, H = 800, 600
if '--size' in a:
    i = a.index('--size'); W, H = map(int, a[i + 1].split('x')); del a[i:i + 2]
os.makedirs(out, exist_ok=True)
LAST_PG = """() => { const l = self.g_trace_lines || []; for (let i = l.length - 1; i >= 0 && i > l.length - 2000; i--) { const m = l[i].match(/"t":"page".*?"pg":(\\d+)|"pg":(\\d+).*?"t":"page"/); if (m) return +(m[1] || m[2]); } return 0; }"""
HAS_LABEL = """(n) => (self.g_trace_lines || []).some(l => l.includes('"t":"label"') && l.includes('"' + n + '"'))"""


def page_of(b):
    for _ in range(120):
        pages = [pg for c in b.contexts for pg in c.pages if 'conscripter' in pg.url]
        if pages:
            return pages[0]
        time.sleep(0.5)
    sys.exit('no page')


def wait(page, js, arg, limit=120):
    t = time.time()
    while time.time() - t < limit:
        try:
            if page.evaluate(js, arg) if arg is not None else page.evaluate(js):
                return True
        except Exception:
            pass
        time.sleep(0.25)
    return False


def at(page, x, y):
    box = page.locator('#canvas').bounding_box()
    return box['x'] + x * box['width'] / W, box['y'] + y * box['height'] / H


with sync_playwright() as p:
    for _ in range(60):
        try:
            b = p.chromium.connect_over_cdp(f'http://127.0.0.1:{port}'); break
        except Exception:
            time.sleep(1)
    page = page_of(b)
    page.set_viewport_size({'width': W, 'height': H + 200})
    wait(page, "() => !!document.querySelector('#canvas') && (self.g_trace_lines || []).length > 0", None)
    t0 = time.time()
    for step in a:
        kind, _, arg = step.partition(':')
        ok = True
        if kind == 'pg':
            ok = wait(page, f"() => ({LAST_PG})() >= {int(arg)}", None)
        elif kind == 'label':
            ok = wait(page, HAS_LABEL, arg)
        elif kind == 'sleep':
            time.sleep(float(arg))
        elif kind == 'key':
            page.keyboard.press(arg)
        elif kind in ('click', 'rclick', 'move'):
            x, y = at(page, *map(float, arg.split(',')))
            if kind == 'move':
                page.mouse.move(x, y)
            else:
                page.mouse.click(x, y, button='right' if kind == 'rclick' else 'left', delay=80)
        elif kind == 'adv':
            x, y = at(page, W / 2, H * 0.4)
            t = time.time(); ok = False
            while time.time() - t < 600:
                if page.evaluate(LAST_PG) >= int(arg):
                    ok = True; break
                page.mouse.click(x, y, delay=60); time.sleep(int(os.environ.get('ADV_MS', 700)) / 1000)
        elif kind == 'wheel':
            box = page.locator('#canvas').bounding_box()
            page.mouse.move(box['x'] + box['width'] / 2, box['y'] + box['height'] / 2)
            for _ in range(abs(int(arg))):
                page.mouse.wheel(0, 120 if int(arg) > 0 else -120); time.sleep(0.15)
        elif kind == 'shot':
            path = f'{out}/{arg}.png'
            page.locator('#canvas').screenshot(path=path)
            # the canvas as laid out; scaled to the game's size when it isn't 1:1
            os.system(f"magick '{path}' -resize {W}x{H}! '{path}'")
        elif kind == 'reload':
            page.reload()
            page = page_of(b)
            wait(page, "() => (self.g_trace_lines || []).length > 0", None)
        try:
            pg = page.evaluate(LAST_PG)
        except Exception:
            pg = '?'
        print(f'{step}\t{time.time() - t0:.1f}s\tpg {pg}' + ('' if ok else '\tTIMEOUT'), flush=True)
