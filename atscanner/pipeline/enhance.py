"""Image enhancement / cleanup (open-source analogue of enhance + cleanup shaders)."""

from __future__ import annotations

from enum import Enum

import cv2
import numpy as np


class EnhanceMode(str, Enum):
    ORIGINAL = "original"
    COLOR = "color"  # color enhancement
    GRAY = "gray"
    BW = "bw"  # high-contrast B&W (paper cleanup)
    SHARPEN = "sharpen"


def enhance(image_bgr: np.ndarray, mode: EnhanceMode | str = EnhanceMode.COLOR) -> np.ndarray:
    mode = EnhanceMode(mode)
    if mode is EnhanceMode.ORIGINAL:
        return image_bgr.copy()

    if mode is EnhanceMode.COLOR:
        lab = cv2.cvtColor(image_bgr, cv2.COLOR_BGR2LAB)
        l, a, b = cv2.split(lab)
        clahe = cv2.createCLAHE(clipLimit=2.0, tileGridSize=(8, 8))
        l = clahe.apply(l)
        merged = cv2.merge([l, a, b])
        out = cv2.cvtColor(merged, cv2.COLOR_LAB2BGR)
        return _sharpen(out, amount=0.4)

    if mode is EnhanceMode.GRAY:
        gray = cv2.cvtColor(image_bgr, cv2.COLOR_BGR2GRAY)
        return cv2.cvtColor(gray, cv2.COLOR_GRAY2BGR)

    if mode is EnhanceMode.BW:
        gray = cv2.cvtColor(image_bgr, cv2.COLOR_BGR2GRAY)
        gray = cv2.GaussianBlur(gray, (3, 3), 0)
        bw = cv2.adaptiveThreshold(
            gray, 255, cv2.ADAPTIVE_THRESH_GAUSSIAN_C, cv2.THRESH_BINARY, 31, 12
        )
        return cv2.cvtColor(bw, cv2.COLOR_GRAY2BGR)

    if mode is EnhanceMode.SHARPEN:
        return _sharpen(image_bgr, amount=1.0)

    return image_bgr.copy()


def _sharpen(image_bgr: np.ndarray, amount: float = 0.5) -> np.ndarray:
    blur = cv2.GaussianBlur(image_bgr, (0, 0), 1.2)
    return cv2.addWeighted(image_bgr, 1.0 + amount, blur, -amount, 0)


def is_mostly_white_paper(image_bgr: np.ndarray, threshold: float = 0.72) -> bool:
    """Lightweight analogue of native-cleanup Paper.isWhitePaper."""
    gray = cv2.cvtColor(image_bgr, cv2.COLOR_BGR2GRAY)
    return float(np.mean(gray) / 255.0) >= threshold
