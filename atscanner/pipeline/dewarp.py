"""
CPU book-curve dewarp (open-source).

Estimate a vertical warp mesh from text-line baselines, then remap on the PC CPU.
"""

from __future__ import annotations

import cv2
import numpy as np


def dewarp_book_curve(
    image_bgr: np.ndarray,
    *,
    cols: int = 32,
    strength: float = 1.0,
) -> np.ndarray:
    """
    Flatten gentle page curl / gutter curve.

    Best after perspective crop. Pure NumPy/OpenCV on host CPU.
    """
    if image_bgr is None or image_bgr.size == 0:
        raise ValueError("empty image")

    h, w = image_bgr.shape[:2]
    gray = cv2.cvtColor(image_bgr, cv2.COLOR_BGR2GRAY)
    gray = cv2.GaussianBlur(gray, (3, 3), 0)

    # Emphasize text strokes
    binary = cv2.adaptiveThreshold(
        gray, 255, cv2.ADAPTIVE_THRESH_GAUSSIAN_C, cv2.THRESH_BINARY_INV, 35, 11
    )
    binary = cv2.morphologyEx(binary, cv2.MORPH_OPEN, np.ones((2, 2), np.uint8), iterations=1)

    cols = max(8, min(cols, w // 8))
    col_w = w / cols
    shifts = np.zeros(cols, dtype=np.float32)

    for i in range(cols):
        x0 = int(i * col_w)
        x1 = int((i + 1) * col_w)
        strip = binary[:, x0:x1]
        # Horizontal projection → find dense text rows, fit a baseline trend
        proj = strip.sum(axis=1).astype(np.float32)
        if proj.max() < 50:
            shifts[i] = 0.0
            continue
        # Local peaks = text lines; use centroid of upper half of ink mass as proxy
        ys = np.where(proj > proj.max() * 0.35)[0]
        if len(ys) < 5:
            shifts[i] = 0.0
            continue
        # Compare column ink centroid to image mid — curl pushes text up/down
        centroid = float(ys.mean())
        shifts[i] = centroid - (h * 0.45)

    # Smooth and zero-mean the shift field
    shifts = cv2.GaussianBlur(shifts.reshape(1, -1), (0, 0), sigmaX=2.0).ravel()
    shifts -= float(np.median(shifts))
    shifts *= float(np.clip(strength, 0.0, 2.5))

    # Build remap grid: interpolate column shifts across width
    xs = (np.arange(cols, dtype=np.float32) + 0.5) * col_w
    map_x = np.tile(np.arange(w, dtype=np.float32), (h, 1))
    map_y = np.tile(np.arange(h, dtype=np.float32).reshape(-1, 1), (1, w))

    shift_row = np.interp(np.arange(w, dtype=np.float32), xs, shifts).astype(np.float32)
    map_y = map_y + shift_row.reshape(1, -1)

    # Mild horizontal un-squeeze near left/right (gutter / edge)
    edge = np.abs(np.linspace(-1.0, 1.0, w, dtype=np.float32))
    squeeze = 1.0 + 0.03 * strength * (edge**2)
    map_x = (map_x - (w * 0.5)) * squeeze.reshape(1, -1) + (w * 0.5)

    return cv2.remap(
        image_bgr,
        map_x,
        map_y,
        interpolation=cv2.INTER_CUBIC,
        borderMode=cv2.BORDER_REPLICATE,
    )
