"""Export scanned pages to PDF / images (ExportPdf* analogue)."""

from __future__ import annotations

from pathlib import Path

import cv2
import img2pdf
import numpy as np
from PIL import Image


def pages_to_pdf(image_paths: list[Path | str], output: Path | str) -> Path:
    paths = [Path(p) for p in image_paths]
    missing = [p for p in paths if not p.is_file()]
    if missing:
        raise FileNotFoundError(f"Missing pages: {missing}")
    out = Path(output)
    out.parent.mkdir(parents=True, exist_ok=True)
    with open(out, "wb") as f:
        f.write(img2pdf.convert([str(p) for p in paths]))
    return out


def bgr_to_png(image_bgr: np.ndarray, output: Path | str) -> Path:
    out = Path(output)
    out.parent.mkdir(parents=True, exist_ok=True)
    rgb = cv2.cvtColor(image_bgr, cv2.COLOR_BGR2RGB)
    Image.fromarray(rgb).save(out, format="PNG")
    return out
