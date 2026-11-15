#!/usr/bin/env python3
"""Build Stabled's background theme gallery from a folder of categories.

    python3 tools/build_backgrounds.py "<source folder>"

The source folder has one sub-folder per category ("Dragon Ball", "Naruto", "general", ...) holding images
(PNG, JPEG or WebP). For each image this writes

  launcher/src-tauri/resources/backgrounds/<category>--<n>.jpg   full background, longest edge <= 1920 px
  launcher/public/backgrounds/<category>--<n>.jpg                 thumbnail, 480 px wide, for the gallery UI

and launcher/src/lib/backgrounds.json, the manifest read by both the UI and the Rust backend. A copy goes
next to the full images as index.json for the in-game client, which receives that folder as
-Dstabled.galleryDir. Existing output is replaced, so the gallery always matches the source folder.
Requires Pillow.
"""
import json
import re
import shutil
import sys
from pathlib import Path

from PIL import Image, ImageOps

ROOT = Path(__file__).resolve().parent.parent
FULL_DIR = ROOT / "launcher/src-tauri/resources/backgrounds"
THUMB_DIR = ROOT / "launcher/public/backgrounds"
MANIFEST = ROOT / "launcher/src/lib/backgrounds.json"
FULL_EDGE = 1920
THUMB_WIDTH = 480
EXTENSIONS = {".png", ".jpg", ".jpeg", ".webp"}
# Display names for folders whose name isn't already presentable.
NAMES = {"general": "General"}
# Categories listed last regardless of name.
LAST = {"general"}


def slug(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")


def save_jpeg(img: Image.Image, path: Path, quality: int) -> None:
    img.convert("RGB").save(path, "JPEG", quality=quality, optimize=True, progressive=True)


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    source = Path(sys.argv[1])
    folders = sorted((d for d in source.iterdir() if d.is_dir()), key=lambda d: (d.name.lower() in LAST, d.name.lower()))
    for d in (FULL_DIR, THUMB_DIR):
        shutil.rmtree(d, ignore_errors=True)
        d.mkdir(parents=True)

    categories = []
    for folder in folders:
        files = sorted(f for f in folder.iterdir() if f.suffix.lower() in EXTENSIONS)
        if not files:
            continue
        cat_id = slug(folder.name)
        images = []
        for n, f in enumerate(files, start=1):
            image_id = f"{n:02d}"
            with Image.open(f) as raw:
                img = ImageOps.exif_transpose(raw)
                img.load()
            full = img.copy()
            full.thumbnail((FULL_EDGE, FULL_EDGE), Image.LANCZOS)
            save_jpeg(full, FULL_DIR / f"{cat_id}--{image_id}.jpg", 86)
            thumb = img.copy()
            thumb.thumbnail((THUMB_WIDTH, THUMB_WIDTH * 2), Image.LANCZOS)
            save_jpeg(thumb, THUMB_DIR / f"{cat_id}--{image_id}.jpg", 80)
            images.append({"id": image_id, "width": full.width, "height": full.height})
        categories.append({"id": cat_id, "name": NAMES.get(folder.name.lower(), folder.name), "images": images})
        print(f"{folder.name}: {len(images)} image(s)")

    manifest = json.dumps({"version": 1, "categories": categories}, indent=2) + "\n"
    MANIFEST.write_text(manifest)
    (FULL_DIR / "index.json").write_text(manifest)
    total = sum(f.stat().st_size for f in FULL_DIR.iterdir()) + sum(f.stat().st_size for f in THUMB_DIR.iterdir())
    print(f"{sum(len(c['images']) for c in categories)} images in {len(categories)} categories, {total / 1e6:.1f} MB")
    return 0


if __name__ == "__main__":
    sys.exit(main())
