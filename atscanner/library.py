"""Local folder/page library."""

from __future__ import annotations

import json
import re
import time
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path


@dataclass
class Page:
    id: str
    path: str
    created_at: float
    note: str = ""


@dataclass
class Folder:
    id: str
    title: str
    created_at: float
    pages: list[Page] = field(default_factory=list)


class Library:
    def __init__(self, root: Path | str | None = None) -> None:
        self.root = Path(root or Path.cwd() / ".library")
        self.root.mkdir(parents=True, exist_ok=True)
        self.meta_path = self.root / "library.json"
        self.folders: list[Folder] = []
        self.load()

    def load(self) -> None:
        if not self.meta_path.is_file():
            self.folders = []
            return
        data = json.loads(self.meta_path.read_text(encoding="utf-8"))
        self.folders = [
            Folder(
                id=f["id"],
                title=f["title"],
                created_at=f["created_at"],
                pages=[Page(**p) for p in f.get("pages", [])],
            )
            for f in data.get("folders", [])
        ]

    def save(self) -> None:
        payload = {"folders": [asdict(f) for f in self.folders]}
        self.meta_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")

    def new_folder(self, title: str = "New Folder") -> Folder:
        folder = Folder(id=str(uuid.uuid4())[:8], title=title, created_at=time.time())
        self.folders.insert(0, folder)
        (self.root / folder.id).mkdir(parents=True, exist_ok=True)
        self.save()
        return folder

    def get_folder(self, folder_id: str) -> Folder:
        for f in self.folders:
            if f.id == folder_id:
                return f
        raise KeyError(folder_id)

    def add_page(self, folder_id: str, image_bytes_png: bytes, note: str = "") -> Page:
        folder = self.get_folder(folder_id)
        page_id = str(uuid.uuid4())[:8]
        rel = f"{folder.id}/{page_id}.png"
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(image_bytes_png)
        page = Page(id=page_id, path=rel, created_at=time.time(), note=note)
        folder.pages.append(page)
        self.save()
        return page

    def page_abs(self, page: Page) -> Path:
        return self.root / page.path

    def delete_folder(self, folder_id: str) -> None:
        import shutil

        folder = self.get_folder(folder_id)
        folder_dir = self.root / folder.id
        if folder_dir.is_dir():
            shutil.rmtree(folder_dir, ignore_errors=True)
        self.folders = [f for f in self.folders if f.id != folder_id]
        self.save()

    def rename_folder(self, folder_id: str, title: str) -> None:
        folder = self.get_folder(folder_id)
        folder.title = _safe_title(title)
        self.save()


def _safe_title(title: str) -> str:
    title = title.strip() or "Untitled"
    return re.sub(r"[\r\n\t]+", " ", title)[:120]
