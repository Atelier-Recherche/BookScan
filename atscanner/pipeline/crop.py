"""Perspective crop / warp (open-source analogue of native-crop + pptp planar straighten)."""

from __future__ import annotations

import cv2
import numpy as np

from .detect import order_corners


def _dest_size(corners: np.ndarray) -> tuple[int, int]:
    tl, tr, br, bl = corners
    width_a = np.linalg.norm(br - bl)
    width_b = np.linalg.norm(tr - tl)
    height_a = np.linalg.norm(tr - br)
    height_b = np.linalg.norm(tl - bl)
    w = max(int(width_a), int(width_b), 1)
    h = max(int(height_a), int(height_b), 1)
    # Cap absurd sizes from bad quads
    w = min(w, 5000)
    h = min(h, 5000)
    return w, h


def perspective_crop(image_bgr: np.ndarray, corners: np.ndarray) -> np.ndarray:
    """Warp document quad to a flat rectangle."""
    src = order_corners(np.asarray(corners, dtype=np.float32))
    w, h = _dest_size(src)
    dst = np.array([[0, 0], [w - 1, 0], [w - 1, h - 1], [0, h - 1]], dtype=np.float32)
    matrix = cv2.getPerspectiveTransform(src, dst)
    return cv2.warpPerspective(image_bgr, matrix, (w, h), flags=cv2.INTER_CUBIC)


def apply_matrix_alter(image_bgr: np.ndarray, matrix_3x3: np.ndarray, out_size: tuple[int, int] | None = None) -> np.ndarray:
    """Apply an affine/perspective 3x3 matrix (similar role to CropTask.applyMatrixAlter)."""
    h, w = image_bgr.shape[:2]
    size = out_size or (w, h)
    return cv2.warpPerspective(image_bgr, matrix_3x3.astype(np.float64), size, flags=cv2.INTER_LINEAR)
