"""OCR helper (local Tesseract if installed; otherwise clear error)."""

from __future__ import annotations

from pathlib import Path

import cv2
import numpy as np


def recognize_text(image_bgr: np.ndarray, lang: str = "eng+fra") -> str:
    try:
        import pytesseract
    except ImportError as exc:
        raise RuntimeError("pytesseract is not installed") from exc

    rgb = cv2.cvtColor(image_bgr, cv2.COLOR_BGR2RGB)
    try:
        return pytesseract.image_to_string(rgb, lang=lang).strip()
    except Exception as exc:  # noqa: BLE001 — surface install guidance
        raise RuntimeError(
            "Tesseract OCR binary not found or failed. "
            "Install from https://github.com/UB-Mannheim/tesseract/wiki "
            "and ensure it is on PATH."
        ) from exc


def recognize_file(path: Path | str, lang: str = "eng+fra") -> str:
    image = cv2.imread(str(path))
    if image is None:
        raise FileNotFoundError(path)
    return recognize_text(image, lang=lang)
