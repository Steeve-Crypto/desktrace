from __future__ import annotations

import os
import platform
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import psutil
from PIL import Image, ImageDraw, ImageFont


HIDDEN_NAME_BITS = (
    "system",
    "idle",
    "registry",
    "svchost",
    "csrss",
    "smss",
    "wininit",
    "services",
    "lsass",
    "fontdrvhost",
    "dwm",
    "conhost",
    "runtimebroker",
    "searchhost",
    "startmenuexperiencehost",
    "textinputhost",
    "securityhealthservice",
)


def list_apps(limit: int = 40) -> tuple[list[dict[str, Any]], str | None]:
    """Return visible-ish user processes and a best-effort focused name."""
    apps: list[dict[str, Any]] = []
    seen: set[str] = set()
    for proc in psutil.process_iter(["pid", "name", "exe", "username"]):
        try:
            info = proc.info
            name = (info.get("name") or "").strip()
            if not name:
                continue
            key = name.lower()
            if any(bit in key for bit in HIDDEN_NAME_BITS):
                continue
            if key in seen:
                continue
            seen.add(key)
            apps.append(
                {
                    "name": name,
                    "pid": info.get("pid"),
                    "exe": info.get("exe"),
                }
            )
        except (psutil.Error, TypeError, ValueError):
            continue
        if len(apps) >= limit:
            break
    apps.sort(key=lambda a: a["name"].lower())
    focused = apps[0]["name"] if apps else None
    return apps, focused


def read_clipboard() -> str | None:
    try:
        import pyperclip

        text = pyperclip.paste()
        if not text:
            return None
        return text[:8000]
    except Exception:
        return None


def _placeholder_image(dest: Path, reason: str) -> None:
    img = Image.new("RGB", (1280, 720), (18, 18, 22))
    draw = ImageDraw.Draw(img)
    try:
        font = ImageFont.load_default()
    except Exception:
        font = None
    lines = [
        "DeskTrace",
        "No live display capture on this session.",
        reason,
        datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC"),
        f"Host: {platform.node()} · {platform.system()}",
    ]
    y = 240
    for line in lines:
        draw.text((80, y), line, fill=(220, 220, 228), font=font)
        y += 36
    img.save(dest, "JPEG", quality=82)


def take_screenshot(dest: Path) -> bool:
    dest.parent.mkdir(parents=True, exist_ok=True)
    if os.environ.get("DESKTRACE_FORCE_PLACEHOLDER") == "1":
        _placeholder_image(dest, "DESKTRACE_FORCE_PLACEHOLDER=1")
        return False
    try:
        import mss

        with mss.mss() as sct:
            monitor = sct.monitors[1] if len(sct.monitors) > 1 else sct.monitors[0]
            raw = sct.grab(monitor)
            img = Image.frombytes("RGB", raw.size, raw.rgb)
            img = img.convert("RGB")
            max_w = 1600
            if img.width > max_w:
                ratio = max_w / img.width
                img = img.resize((max_w, int(img.height * ratio)))
            img.save(dest, "JPEG", quality=72, optimize=True)
            return True
    except Exception as exc:
        _placeholder_image(dest, f"capture fallback: {type(exc).__name__}")
        return False


def snapshot_files(store_shots: Path) -> tuple[Path, bool]:
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    dest = store_shots / f"{stamp}.jpg"
    real = take_screenshot(dest)
    return dest, real
