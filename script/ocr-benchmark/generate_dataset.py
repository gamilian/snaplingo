"""Generate deterministic screenshot-shaped workloads; these are not accuracy fixtures."""
import argparse
import hashlib
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    font_path = "/System/Library/Fonts/Hiragino Sans GB.ttc"
    cases = [
        ("small", 512, 100, 20, 1),
        ("compact", 960, 320, 20, 1),
        ("document", 1280, 960, 24, 1),
        ("dense", 1920, 1080, 28, 2),
        ("4k", 3840, 2160, 40, 3),
        ("wide", 2400, 180, 28, 1),
    ]
    texts = [
        "设置 Settings  中英文截图 OCR  Version 2026.11",
        "文件 File / 编辑 Edit / 视图 View / 帮助 Help",
        "离线识别 Offline recognition with GPU acceleration",
        "模型 Model: PP-OCR  内存 Memory: 128 MB",
        "https://example.com/docs  用户 User: snaplingo",
        "const result = await recognize(image); // 识别结果",
    ]
    manifest = []
    for category, width, height, size, columns in cases:
        font = ImageFont.truetype(font_path, size)
        for theme in ("light", "dark"):
            background, foreground = (("#f5f6f8", "#20242a") if theme == "light"
                                       else ("#1b2028", "#e8edf4"))
            image = Image.new("RGB", (width, height), background)
            draw = ImageDraw.Draw(image)
            count = 0
            column_width = width // columns
            for column in range(columns):
                for row, y in enumerate(range(10, height - size - 4, size + 10)):
                    text = f"{row + 1:02d} " + texts[(row + column) % len(texts)]
                    while draw.textlength(text, font=font) > column_width - 24:
                        text = text[:-1]
                    draw.text((column * column_width + 10, y), text, font=font,
                              fill=foreground)
                    count += 1
            name = f"{category}-{theme}"
            path = args.output / f"{name}.png"
            image.save(path)
            manifest.append({"id": name, "image": str(path.resolve()),
                             "category": category, "theme": theme,
                             "width": width, "height": height, "line_count": count,
                             "font_px": size, "font": font_path,
                             "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
    (args.output / "manifest.jsonl").write_text(
        "".join(json.dumps(row, ensure_ascii=False) + "\n" for row in manifest))
    (args.output / "probe.jsonl").write_text("".join(
        json.dumps(row, ensure_ascii=False) + "\n" for row in manifest
        if row["id"] in ("small-light", "document-light")))
    print(json.dumps({"images": len(manifest), "output": str(args.output),
                      "purpose": "performance workloads only; no accuracy ranking"}))


if __name__ == "__main__":
    main()
