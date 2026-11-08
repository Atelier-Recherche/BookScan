"""
ATscanner desktop UI (deprecated prototype).

Flow: capture/import → detect quad → perspective crop → enhance → library → PDF / OCR
"""

from __future__ import annotations

import io
import sys
import tkinter as tk
from pathlib import Path
from tkinter import filedialog, messagebox, simpledialog, ttk

import cv2
import numpy as np
from PIL import Image, ImageTk

from atscanner.library import Library
from atscanner.pipeline import (
    EnhanceMode,
    default_full_frame_quad,
    dewarp_book_curve,
    draw_quad,
    enhance,
    find_document_quad,
    pages_to_pdf,
    perspective_crop,
    recognize_text,
    remove_fingers,
    remove_handwriting,
)


APP_TITLE = "ATscanner"
ACCENT = "#32CDB1"


class ScannerApp(tk.Tk):
    def __init__(self) -> None:
        super().__init__()
        self.title(APP_TITLE)
        self.geometry("1180x720")
        self.minsize(960, 600)
        self.configure(bg="#121417")

        self.library = Library(Path.cwd() / ".library")
        self.source_bgr: np.ndarray | None = None
        self.corners: np.ndarray | None = None
        self.result_bgr: np.ndarray | None = None
        self.enhance_mode = tk.StringVar(value=EnhanceMode.COLOR.value)
        self.do_dewarp = tk.BooleanVar(value=True)
        self.do_finger = tk.BooleanVar(value=True)
        self.do_handwriting = tk.BooleanVar(value=False)
        self.folder_var = tk.StringVar()
        self.status_var = tk.StringVar(
            value="Prêt — traitement 100% CPU PC (pas sur le téléphone)."
        )
        self._photo_left = None
        self._photo_right = None
        self._cam = None
        self._cam_job = None

        self._build_style()
        self._build_ui()
        self._refresh_folders()

    def _build_style(self) -> None:
        style = ttk.Style(self)
        try:
            style.theme_use("clam")
        except tk.TclError:
            pass
        style.configure("TFrame", background="#121417")
        style.configure("Card.TFrame", background="#1A1E24")
        style.configure("TLabel", background="#121417", foreground="#E8ECEF")
        style.configure("Muted.TLabel", background="#121417", foreground="#9AA3AD")
        style.configure("Brand.TLabel", background="#121417", foreground=ACCENT, font=("Segoe UI Semibold", 18))
        style.configure("TButton", padding=8)
        style.configure("Accent.TButton", background=ACCENT, foreground="#0B0D0F")
        style.map("Accent.TButton", background=[("active", "#2BB89E")])

    def _build_ui(self) -> None:
        top = ttk.Frame(self, padding=12)
        top.pack(fill=tk.X)
        ttk.Label(top, text="ATscanner", style="Brand.TLabel").pack(side=tk.LEFT)
        ttk.Label(
            top,
            text="  PC CPU: detect → crop → dewarp → finger → enhance → PDF",
            style="Muted.TLabel",
        ).pack(side=tk.LEFT, padx=8)

        body = ttk.Frame(self, padding=(12, 0, 12, 12))
        body.pack(fill=tk.BOTH, expand=True)
        body.columnconfigure(0, weight=1)
        body.columnconfigure(1, weight=1)
        body.rowconfigure(0, weight=1)

        left = ttk.Frame(body, style="Card.TFrame", padding=8)
        left.grid(row=0, column=0, sticky="nsew", padx=(0, 6))
        right = ttk.Frame(body, style="Card.TFrame", padding=8)
        right.grid(row=0, column=1, sticky="nsew", padx=(6, 0))

        ttk.Label(left, text="Source + détection", background="#1A1E24").pack(anchor=tk.W)
        self.canvas_left = tk.Label(left, bg="#0E1013", fg="#E8ECEF")
        self.canvas_left.pack(fill=tk.BOTH, expand=True, pady=8)

        ttk.Label(right, text="Résultat (CPU PC)", background="#1A1E24").pack(anchor=tk.W)
        self.canvas_right = tk.Label(right, bg="#0E1013", fg="#E8ECEF")
        self.canvas_right.pack(fill=tk.BOTH, expand=True, pady=8)

        opts = ttk.Frame(self, padding=(12, 0, 12, 0))
        opts.pack(fill=tk.X)
        ttk.Checkbutton(opts, text="Dewarp courbe (livre)", variable=self.do_dewarp).pack(side=tk.LEFT, padx=4)
        ttk.Checkbutton(opts, text="Finger remove", variable=self.do_finger).pack(side=tk.LEFT, padx=4)
        ttk.Checkbutton(opts, text="Handwriting erase", variable=self.do_handwriting).pack(side=tk.LEFT, padx=4)
        ttk.Label(opts, text="← tout tourne sur le CPU de l'ordi", style="Muted.TLabel").pack(side=tk.LEFT, padx=8)

        tools = ttk.Frame(self, padding=12)
        tools.pack(fill=tk.X)

        ttk.Button(tools, text="Importer image", command=self.import_image).pack(side=tk.LEFT, padx=2)
        ttk.Button(tools, text="Caméra", command=self.toggle_camera).pack(side=tk.LEFT, padx=2)
        ttk.Button(tools, text="Capturer", command=self.capture_frame).pack(side=tk.LEFT, padx=2)
        ttk.Button(tools, text="Redétecter", command=self.redetect).pack(side=tk.LEFT, padx=2)
        ttk.Button(tools, text="Traiter (CPU)", command=self.apply_pipeline, style="Accent.TButton").pack(
            side=tk.LEFT, padx=8
        )

        ttk.Label(tools, text="Enhance").pack(side=tk.LEFT, padx=(12, 4))
        modes = ttk.Combobox(
            tools,
            textvariable=self.enhance_mode,
            values=[m.value for m in EnhanceMode],
            width=10,
            state="readonly",
        )
        modes.pack(side=tk.LEFT)
        modes.bind("<<ComboboxSelected>>", lambda _e: self.apply_pipeline())

        ttk.Label(tools, text="Dossier").pack(side=tk.LEFT, padx=(16, 4))
        self.folder_combo = ttk.Combobox(tools, textvariable=self.folder_var, width=24, state="readonly")
        self.folder_combo.pack(side=tk.LEFT)
        ttk.Button(tools, text="+", width=3, command=self.create_folder).pack(side=tk.LEFT, padx=2)
        ttk.Button(tools, text="Sauver page", command=self.save_page).pack(side=tk.LEFT, padx=6)
        ttk.Button(tools, text="Export PDF", command=self.export_pdf).pack(side=tk.LEFT, padx=2)
        ttk.Button(tools, text="OCR", command=self.run_ocr).pack(side=tk.LEFT, padx=2)

        status = ttk.Label(self, textvariable=self.status_var, style="Muted.TLabel", padding=(12, 0, 12, 10))
        status.pack(fill=tk.X)

        self.protocol("WM_DELETE_WINDOW", self.on_close)

    def _refresh_folders(self) -> None:
        if not self.library.folders:
            self.library.new_folder("Inbox")
        labels = [f"{f.title} ({f.id})" for f in self.library.folders]
        self.folder_combo["values"] = labels
        self.folder_var.set(labels[0])

    def _current_folder_id(self) -> str:
        label = self.folder_var.get()
        return label.rsplit("(", 1)[-1].rstrip(")")

    def import_image(self) -> None:
        path = filedialog.askopenfilename(
            title="Importer une image",
            filetypes=[("Images", "*.png;*.jpg;*.jpeg;*.webp;*.bmp;*.tif;*.tiff"), ("Tous", "*.*")],
        )
        if not path:
            return
        self.stop_camera()
        image = cv2.imread(path)
        if image is None:
            messagebox.showerror(APP_TITLE, "Impossible de lire l'image.")
            return
        self.source_bgr = image
        self.redetect()
        self.status_var.set(f"Importé : {Path(path).name}")

    def toggle_camera(self) -> None:
        if self._cam is not None:
            self.stop_camera()
            self.status_var.set("Caméra arrêtée.")
            return
        cam = cv2.VideoCapture(0, cv2.CAP_DSHOW)
        if not cam.isOpened():
            cam = cv2.VideoCapture(0)
        if not cam.isOpened():
            messagebox.showerror(APP_TITLE, "Aucune caméra disponible.")
            return
        self._cam = cam
        self.status_var.set("Caméra active — Capturer pour figer une frame.")
        self._poll_camera()

    def _poll_camera(self) -> None:
        if self._cam is None:
            return
        ok, frame = self._cam.read()
        if ok:
            self.source_bgr = frame
            corners = find_document_quad(frame) or default_full_frame_quad(frame)
            self.corners = corners
            self._show_left(draw_quad(frame, corners))
        self._cam_job = self.after(33, self._poll_camera)

    def capture_frame(self) -> None:
        if self.source_bgr is None:
            messagebox.showinfo(APP_TITLE, "Aucune frame à capturer.")
            return
        self.source_bgr = self.source_bgr.copy()
        self.stop_camera()
        self.redetect()
        self.apply_pipeline()
        self.status_var.set("Frame capturée.")

    def stop_camera(self) -> None:
        if self._cam_job is not None:
            self.after_cancel(self._cam_job)
            self._cam_job = None
        if self._cam is not None:
            self._cam.release()
            self._cam = None

    def redetect(self) -> None:
        if self.source_bgr is None:
            return
        corners = find_document_quad(self.source_bgr)
        if corners is None:
            corners = default_full_frame_quad(self.source_bgr)
            self.status_var.set("Aucun document auto-détecté — cadre par défaut (sélection manuelle).")
        else:
            self.status_var.set("Document détecté (quad 4 coins).")
        self.corners = corners
        self._show_left(draw_quad(self.source_bgr, corners))

    def apply_pipeline(self) -> None:
        if self.source_bgr is None or self.corners is None:
            messagebox.showinfo(APP_TITLE, "Charge d'abord une image.")
            return
        self.status_var.set("Traitement CPU en cours…")
        self.update_idletasks()

        img = perspective_crop(self.source_bgr, self.corners)
        steps = ["crop"]

        if self.do_dewarp.get():
            img = dewarp_book_curve(img)
            steps.append("dewarp")

        if self.do_finger.get():
            img, _mask = remove_fingers(img)
            steps.append("finger")

        if self.do_handwriting.get():
            img, _mask = remove_handwriting(img)
            steps.append("handwriting")

        img = enhance(img, self.enhance_mode.get())
        steps.append(f"enhance:{self.enhance_mode.get()}")

        self.result_bgr = img
        self._show_right(self.result_bgr)
        self.status_var.set(
            f"CPU OK [{'+'.join(steps)}] · {self.result_bgr.shape[1]}x{self.result_bgr.shape[0]}"
        )

    def create_folder(self) -> None:
        title = simpledialog.askstring(APP_TITLE, "Nom du dossier / carnet :", parent=self)
        if not title:
            return
        self.library.new_folder(title)
        self._refresh_folders()
        self.folder_var.set(f"{self.library.folders[0].title} ({self.library.folders[0].id})")

    def save_page(self) -> None:
        if self.result_bgr is None:
            self.apply_pipeline()
        if self.result_bgr is None:
            return
        ok, buf = cv2.imencode(".png", self.result_bgr)
        if not ok:
            messagebox.showerror(APP_TITLE, "Échec encodage PNG.")
            return
        page = self.library.add_page(self._current_folder_id(), buf.tobytes())
        self.status_var.set(f"Page sauvée : {page.path}")
        self._refresh_folders()

    def export_pdf(self) -> None:
        try:
            folder = self.library.get_folder(self._current_folder_id())
        except KeyError:
            messagebox.showerror(APP_TITLE, "Dossier introuvable.")
            return
        if not folder.pages:
            messagebox.showinfo(APP_TITLE, "Aucune page dans ce dossier.")
            return
        out = filedialog.asksaveasfilename(
            defaultextension=".pdf",
            filetypes=[("PDF", "*.pdf")],
            initialfile=f"{folder.title}.pdf",
        )
        if not out:
            return
        paths = [self.library.page_abs(p) for p in folder.pages]
        pages_to_pdf(paths, out)
        self.status_var.set(f"PDF exporté : {out}")
        messagebox.showinfo(APP_TITLE, f"PDF créé\n{out}")

    def run_ocr(self) -> None:
        if self.result_bgr is None:
            self.apply_pipeline()
        if self.result_bgr is None:
            return
        try:
            text = recognize_text(self.result_bgr)
        except RuntimeError as exc:
            messagebox.showerror(APP_TITLE, str(exc))
            return
        win = tk.Toplevel(self)
        win.title("OCR")
        win.geometry("640x420")
        txt = tk.Text(win, wrap=tk.WORD)
        txt.pack(fill=tk.BOTH, expand=True)
        txt.insert("1.0", text or "(aucun texte reconnu)")

    def _show_left(self, image_bgr: np.ndarray) -> None:
        self._photo_left = self._to_photo(image_bgr, self.canvas_left)
        self.canvas_left.configure(image=self._photo_left)

    def _show_right(self, image_bgr: np.ndarray) -> None:
        self._photo_right = self._to_photo(image_bgr, self.canvas_right)
        self.canvas_right.configure(image=self._photo_right)

    def _to_photo(self, image_bgr: np.ndarray, widget: tk.Label) -> ImageTk.PhotoImage:
        widget.update_idletasks()
        max_w = max(widget.winfo_width(), 480)
        max_h = max(widget.winfo_height(), 360)
        rgb = cv2.cvtColor(image_bgr, cv2.COLOR_BGR2RGB)
        h, w = rgb.shape[:2]
        scale = min(max_w / w, max_h / h, 1.0)
        if scale < 1.0:
            rgb = cv2.resize(rgb, (int(w * scale), int(h * scale)), interpolation=cv2.INTER_AREA)
        return ImageTk.PhotoImage(Image.fromarray(rgb))

    def on_close(self) -> None:
        self.stop_camera()
        self.destroy()


def main() -> int:
    app = ScannerApp()
    app.mainloop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
