"""CPU finger removal via skin segmentation + inpainting (open-source)."""

from __future__ import annotations

import cv2
import numpy as np


def skin_mask_bgr(image_bgr: np.ndarray) -> np.ndarray:
    """Detect likely skin pixels (YCrCb + HSV intersection)."""
    ycrcb = cv2.cvtColor(image_bgr, cv2.COLOR_BGR2YCrCb)
    hsv = cv2.cvtColor(image_bgr, cv2.COLOR_BGR2HSV)

    # Typical skin ranges (works for many lighting conditions; not ML)
    mask_y = cv2.inRange(ycrcb, (0, 133, 77), (255, 173, 127))
    mask_h = cv2.inRange(hsv, (0, 30, 50), (25, 180, 255))
    mask = cv2.bitwise_and(mask_y, mask_h)

    # Prefer bottom/side regions (fingers holding page) — reduce false positives on photos in doc
    h, w = mask.shape
    weight = np.zeros((h, w), dtype=np.float32)
    weight[int(h * 0.55) :, :] = 1.0
    weight[:, : int(w * 0.12)] = 1.0
    weight[:, int(w * 0.88) :] = 1.0
    weight = cv2.GaussianBlur(weight, (0, 0), sigmaX=w * 0.05)
    mask = ((mask.astype(np.float32) / 255.0) * (0.35 + 0.65 * weight) > 0.5).astype(np.uint8) * 255

    kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (9, 9))
    mask = cv2.morphologyEx(mask, cv2.MORPH_CLOSE, kernel, iterations=2)
    mask = cv2.morphologyEx(mask, cv2.MORPH_OPEN, kernel, iterations=1)

    # Drop tiny blobs
    num, labels, stats, _ = cv2.connectedComponentsWithStats(mask, connectivity=8)
    cleaned = np.zeros_like(mask)
    min_area = (h * w) * 0.002
    for i in range(1, num):
        if stats[i, cv2.CC_STAT_AREA] >= min_area:
            cleaned[labels == i] = 255
    return cleaned


def remove_fingers(image_bgr: np.ndarray, *, radius: int = 5) -> tuple[np.ndarray, np.ndarray]:
    """
    Returns (cleaned_image, mask_used).
    Inpaint runs on host CPU — slower than phone GPU but no thermal throttle on the phone.
    """
    mask = skin_mask_bgr(image_bgr)
    if int(cv2.countNonZero(mask)) == 0:
        return image_bgr.copy(), mask
    cleaned = cv2.inpaint(image_bgr, mask, inpaintRadius=radius, flags=cv2.INPAINT_TELEA)
    return cleaned, mask
