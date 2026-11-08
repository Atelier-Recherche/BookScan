"""
CPU scribble / handwriting eraser (open-source heuristic).

Targets pen strokes (blue/black ink, thin connected components) then inpaints.
Heuristic only — runs entirely on the PC CPU.
"""

from __future__ import annotations

import cv2
import numpy as np


def handwriting_mask_bgr(image_bgr: np.ndarray) -> np.ndarray:
    h, w = image_bgr.shape[:2]
    hsv = cv2.cvtColor(image_bgr, cv2.COLOR_BGR2HSV)
    gray = cv2.cvtColor(image_bgr, cv2.COLOR_BGR2GRAY)

    # Blue / purple pen
    blue = cv2.inRange(hsv, (90, 40, 40), (140, 255, 255))
    # Red pen
    red1 = cv2.inRange(hsv, (0, 60, 40), (12, 255, 255))
    red2 = cv2.inRange(hsv, (165, 60, 40), (180, 255, 255))
    # Dark ink on bright paper (black pen), excluding large printed text blocks somewhat
    dark = cv2.adaptiveThreshold(
        gray, 255, cv2.ADAPTIVE_THRESH_GAUSSIAN_C, cv2.THRESH_BINARY_INV, 31, 18
    )

    # Blackhat highlights thin dark strokes
    kernel = cv2.getStructuringElement(cv2.MORPH_RECT, (9, 9))
    blackhat = cv2.morphologyEx(gray, cv2.MORPH_BLACKHAT, kernel)
    _, thin = cv2.threshold(blackhat, 25, 255, cv2.THRESH_BINARY)

    mask = cv2.bitwise_or(blue, cv2.bitwise_or(red1, red2))
    mask = cv2.bitwise_or(mask, cv2.bitwise_and(dark, thin))

    # Keep elongated / small components typical of handwriting, drop big filled regions
    num, labels, stats, _ = cv2.connectedComponentsWithStats(mask, connectivity=8)
    cleaned = np.zeros_like(mask)
    img_area = h * w
    for i in range(1, num):
        area = stats[i, cv2.CC_STAT_AREA]
        bw = stats[i, cv2.CC_STAT_WIDTH]
        bh = stats[i, cv2.CC_STAT_HEIGHT]
        if area < 40 or area > img_area * 0.08:
            continue
        aspect = max(bw, bh) / max(min(bw, bh), 1)
        fill = area / float(max(bw * bh, 1))
        # Handwriting: elongated or sparse fill; printed bold blocks are denser/squarer
        if aspect >= 1.8 or fill < 0.35:
            cleaned[labels == i] = 255

    cleaned = cv2.dilate(cleaned, np.ones((2, 2), np.uint8), iterations=1)
    return cleaned


def remove_handwriting(image_bgr: np.ndarray, *, radius: int = 3) -> tuple[np.ndarray, np.ndarray]:
    mask = handwriting_mask_bgr(image_bgr)
    if int(cv2.countNonZero(mask)) == 0:
        return image_bgr.copy(), mask
    cleaned = cv2.inpaint(image_bgr, mask, inpaintRadius=radius, flags=cv2.INPAINT_TELEA)
    return cleaned, mask
