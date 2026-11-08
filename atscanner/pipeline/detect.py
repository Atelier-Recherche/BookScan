"""Document edge detection (open-source analogue of freeform / page detection)."""

from __future__ import annotations

import cv2
import numpy as np


def order_corners(pts: np.ndarray) -> np.ndarray:
    """Order 4 points: top-left, top-right, bottom-right, bottom-left."""
    pts = pts.reshape(4, 2).astype(np.float32)
    s = pts.sum(axis=1)
    diff = np.diff(pts, axis=1).ravel()
    ordered = np.zeros((4, 2), dtype=np.float32)
    ordered[0] = pts[np.argmin(s)]
    ordered[2] = pts[np.argmax(s)]
    ordered[1] = pts[np.argmin(diff)]
    ordered[3] = pts[np.argmax(diff)]
    return ordered


def find_document_quad(
    image_bgr: np.ndarray,
    *,
    max_side: int = 1280,
) -> np.ndarray | None:
    """
    Detect the largest 4-corner document contour.

    Returns corners in original image coordinates, or None.
    """
    h, w = image_bgr.shape[:2]
    scale = 1.0
    work = image_bgr
    if max(h, w) > max_side:
        scale = max_side / float(max(h, w))
        work = cv2.resize(image_bgr, (int(w * scale), int(h * scale)), interpolation=cv2.INTER_AREA)

    gray = cv2.cvtColor(work, cv2.COLOR_BGR2GRAY)
    gray = cv2.GaussianBlur(gray, (5, 5), 0)
    edges = cv2.Canny(gray, 50, 150)
    edges = cv2.dilate(edges, np.ones((3, 3), np.uint8), iterations=1)

    contours, _ = cv2.findContours(edges, cv2.RETR_LIST, cv2.CHAIN_APPROX_SIMPLE)
    contours = sorted(contours, key=cv2.contourArea, reverse=True)[:12]

    best = None
    best_area = 0.0
    img_area = work.shape[0] * work.shape[1]

    for cnt in contours:
        peri = cv2.arcLength(cnt, True)
        approx = cv2.approxPolyDP(cnt, 0.02 * peri, True)
        if len(approx) != 4:
            continue
        area = cv2.contourArea(approx)
        if area < img_area * 0.12:
            continue
        if area > best_area:
            best_area = area
            best = approx

    if best is None:
        return None

    corners = order_corners(best.astype(np.float32))
    if scale != 1.0:
        corners /= scale
    return corners


def default_full_frame_quad(image_bgr: np.ndarray, margin: float = 0.05) -> np.ndarray:
    """Fallback quad inset from image borders (manual-select analogue)."""
    h, w = image_bgr.shape[:2]
    mx, my = w * margin, h * margin
    return np.array(
        [[mx, my], [w - mx, my], [w - mx, h - my], [mx, h - my]],
        dtype=np.float32,
    )


def draw_quad(image_bgr: np.ndarray, corners: np.ndarray, color=(50, 205, 177), thickness=3) -> np.ndarray:
    out = image_bgr.copy()
    pts = corners.astype(np.int32).reshape(-1, 1, 2)
    cv2.polylines(out, [pts], True, color, thickness)
    for i, (x, y) in enumerate(corners.astype(int)):
        cv2.circle(out, (x, y), 8, color, -1)
        cv2.putText(out, str(i + 1), (x + 10, y - 10), cv2.FONT_HERSHEY_SIMPLEX, 0.6, color, 2)
    return out
