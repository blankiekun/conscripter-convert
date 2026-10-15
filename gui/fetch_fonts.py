"""Downloads IPA Gothic and IPA Mincho (IPAfont 003.03) into gui/fonts/,
with their license, for bundling into the app. The IPA Font License
allows redistribution with the license text and unchanged fonts."""

import hashlib
import io
import sys
import urllib.request
import zipfile
from pathlib import Path

URL = "https://moji.or.jp/wp-content/ipafont/IPAfont/IPAfont00303.zip"
SHA256 = "f755ed79a4b8e715bed2f05a189172138aedf93db0f465b4e20c344a02766fe5"
FILES = ("ipag.ttf", "ipam.ttf", "IPA_Font_License_Agreement_v1.0.txt")


def main() -> int:
    dest = Path(__file__).resolve().parent / "fonts"
    if all((dest / f).is_file() for f in FILES):
        print(f"fonts already in {dest}")
        return 0
    data = urllib.request.urlopen(URL, timeout=120).read()
    digest = hashlib.sha256(data).hexdigest()
    if digest != SHA256:
        print(f"{URL}: sha256 {digest}, expected {SHA256}", file=sys.stderr)
        return 1
    dest.mkdir(exist_ok=True)
    with zipfile.ZipFile(io.BytesIO(data)) as z:
        for f in FILES:
            (dest / f).write_bytes(z.read(f"IPAfont00303/{f}"))
    print(f"wrote {', '.join(FILES)} to {dest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
