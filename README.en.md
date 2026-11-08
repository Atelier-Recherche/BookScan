<div align="center" id="head">

<table>
<tr>
<td><img src="readme-media/logo_scanner.png" alt="BookScan" width="140" /></td>
<td align="left">
<h1 style="margin:0">BookScan</h1>
<p style="margin:0.25em 0 0"><strong>Book scanning · Companion · Android / PC</strong><br/>bookscan · OpenCV / Rust pipeline</p>
</td>
</tr>
</table>

<a href="https://atelier.atechnologie.fr/" title="l'Atelier – book fabrication and research tools association"><img src="https://raw.githubusercontent.com/Atelier-Recherche/.github/main/profile/logo.jpg" alt="l'Atelier" width="200" /></a>  
<sub>Developed by <a href="https://atelier.atechnologie.fr/">l'Atelier</a> — book fabrication and research tools (EHESS)</sub>

<p>
🇫🇷 <a href="README.md">Français</a> ·
🇬🇧 <a href="README.en.md"><b>English</b></a> ·
🇩🇪 <a href="README.de.md">Deutsch</a> ·
🇪🇸 <a href="README.es.md">Español</a>
</p>

<p>
<a href="https://atelier.atechnologie.fr/"><img src="https://img.shields.io/badge/🌐_l'Atelier-atelier.atechnologie.fr-2d5016?style=for-the-badge" alt="l'Atelier website" /></a>

</p>

</div>

---

## 📸 Preview

| Companion | Capture |
| :---: | :---: |
| <img src="readme-media/screen1.jpg" alt="BookScan — Companion" width="400" /> | <img src="readme-media/screen2.jpg" alt="BookScan — Capture" width="400" /> |

## 📖 About

**BookScan** — Atelier toolkit for book scanning: Android **bookscan** app (capture) + desktop **bookscan-companion** (LAN receive, image pipeline, Livescan UI) + utility scripts.

The repo also includes a Tkinter Python prototype `atscanner/` that is **deprecated** (replaced by the Rust companion).

## 🧩 Components

| Piece | Stack | Role |
|-------|-------|------|
| `bookscan/` | Android (Gradle) | Capture (auto / semi / manual), send to companion |
| `bookscan-companion/` | Rust (eframe/egui, axum) | LAN server (port **7842**), CPU pipeline, PDF export, UI |
| `protocol/` | WebSocket JSON | `/v1/ws` schema (capture_now, set_mode, point_state…) |
| `atscanner/` | Python (OpenCV…) | Deprecated UI prototype |
| `scripts/` | bat / Python | Firewall helper, companion smoke test |

## ✨ Pipeline (companion)

Receive pages → document detection (quad) → warp / crop → enhance → local library → PDF export. The companion pipeline also includes finger / handwriting removal stages.

## ⬇️ Install / run

### PC companion

```bat
run_companion.bat
```

- Runs `bookscan-companion/target/release/bookscan-companion.exe` if present
- Otherwise (or with `--rebuild`): `cargo build --release` then run
- Default port: **7842** (`BOOKSCAN_PORT` to override)

Windows firewall (TCP 7842 rule):

```bat
scripts\open-firewall-bookscan.bat
```

### Android bookscan app

```bat
cd bookscan
gradlew.bat assembleDebug
```

Install the APK, keep phone and PC on the same Wi‑Fi, point the app at the companion IP.

### Legacy Python launcher

`run_atscanner.bat` and `python -m atscanner` are **deprecated** and redirect to the companion.

## 📚 Python dependencies (prototype / tools)

`requirements.txt`:

* `opencv-python`, `numpy`, `Pillow`, `img2pdf`, `pytesseract`

## 📜 Scripts

| Script | Role |
|--------|------|
| `run_companion.bat` | Build/run the Rust companion |
| `run_atscanner.bat` | Deprecated → calls `run_companion.bat` |
| `scripts/open-firewall-bookscan.bat` | Opens TCP port 7842 |
| `scripts/smoke_companion.py` | Companion smoke test |

## 🔗 Resources

| | |
| --- | --- |
| 🌐 **l'Atelier** | [atelier.atechnologie.fr](https://atelier.atechnologie.fr/) |

---

<div align="center">

<sub>🇫🇷 <a href="README.md">Français</a> · <a href="README.en.md">🇬🇧 English</a> · <a href="README.de.md">🇩🇪 Deutsch</a> · <a href="README.es.md">🇪🇸 Español</a> · BookScan — <a href="https://atelier.atechnologie.fr/">l'Atelier</a></sub>

</div>
