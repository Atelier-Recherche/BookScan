<div align="center" id="head">

<table>
<tr>
<td><img src="readme-media/logo_scanner.png" alt="BookScan" width="140" /></td>
<td align="left">
<h1 style="margin:0">BookScan</h1>
<p style="margin:0.25em 0 0"><strong>Buchscan · Companion · Android / PC</strong><br/>bookscan · OpenCV- / Rust-Pipeline</p>
</td>
</tr>
</table>

<a href="https://atelier.atechnologie.fr/" title="l'Atelier – Verein für Buchherstellung und Forschungswerkzeuge"><img src="https://raw.githubusercontent.com/Atelier-Recherche/.github/main/profile/logo.jpg" alt="l'Atelier" width="200" /></a>  
<sub>Entwickelt von <a href="https://atelier.atechnologie.fr/">l'Atelier</a> — Buchherstellung und Forschungswerkzeuge (EHESS)</sub>

<p>
🇫🇷 <a href="README.md">Français</a> ·
🇬🇧 <a href="README.en.md">English</a> ·
🇩🇪 <a href="README.de.md"><b>Deutsch</b></a> ·
🇪🇸 <a href="README.es.md">Español</a>
</p>

<p>
<a href="https://atelier.atechnologie.fr/"><img src="https://img.shields.io/badge/🌐_l'Atelier-atelier.atechnologie.fr-2d5016?style=for-the-badge" alt="Website l'Atelier" /></a>

</p>

</div>

---

## 📸 Vorschau

| Companion | Aufnahme |
| :---: | :---: |
| <img src="readme-media/screen1.jpg" alt="BookScan — Companion" width="400" /> | <img src="readme-media/screen2.jpg" alt="BookScan — Aufnahme" width="400" /> |

## 📖 Über

**BookScan** — Atelier-Werkzeugsatz zum Buchscannen: Android-App **bookscan** (Aufnahme) + Desktop-**bookscan-companion** (LAN-Empfang, Bildpipeline, Livescan-UI) + Hilfsskripte.

Das Repository enthält außerdem einen Tkinter-Python-Prototyp `atscanner/`, der **veraltet** ist (ersetzt durch den Rust-Companion).

## 🧩 Komponenten

| Teil | Technik | Rolle |
|------|---------|------|
| `bookscan/` | Android (Gradle) | Aufnahme (auto / semi / manuell), Versand an Companion |
| `bookscan-companion/` | Rust (eframe/egui, axum) | LAN-Server (Port **7842**), CPU-Pipeline, PDF-Export, UI |
| `protocol/` | WebSocket JSON | Schema `/v1/ws` (capture_now, set_mode, point_state…) |
| `atscanner/` | Python (OpenCV…) | Veralteter UI-Prototyp |
| `scripts/` | bat / Python | Firewall-Hilfe, Companion-Smoke-Test |

## ✨ Pipeline (Companion)

Seiten empfangen → Dokumenterkennung (Quad) → Entzerren / Zuschneiden → Verbessern → lokale Bibliothek → PDF-Export. Die Pipeline enthält auch Stufen zur Finger-/Handschriftentfernung.

## ⬇️ Installation / Start

### PC-Companion

```bat
run_companion.bat
```

- Startet `bookscan-companion/target/release/bookscan-companion.exe`, falls vorhanden
- Sonst (oder mit `--rebuild`): `cargo build --release`, dann Start
- Standardport: **7842** (`BOOKSCAN_PORT` zum Ändern)

Windows-Firewall (TCP-7842-Regel):

```bat
scripts\open-firewall-bookscan.bat
```

### Android-App bookscan

```bat
cd bookscan
gradlew.bat assembleDebug
```

APK installieren, Telefon und PC im selben WLAN halten, App auf die Companion-IP zeigen.

### Alter Python-Starter

`run_atscanner.bat` und `python -m atscanner` sind **veraltet** und leiten zum Companion weiter.

## 📚 Python-Abhängigkeiten (Prototyp / Tools)

`requirements.txt`:

* `opencv-python`, `numpy`, `Pillow`, `img2pdf`, `pytesseract`

## 📜 Skripte

| Skript | Rolle |
|--------|------|
| `run_companion.bat` | Build/Start des Rust-Companions |
| `run_atscanner.bat` | Veraltet → ruft `run_companion.bat` auf |
| `scripts/open-firewall-bookscan.bat` | Öffnet TCP-Port 7842 |
| `scripts/smoke_companion.py` | Companion-Smoke-Test |

## 🔗 Ressourcen

| | |
| --- | --- |
| 🌐 **l'Atelier** | [atelier.atechnologie.fr](https://atelier.atechnologie.fr/) |

---

<div align="center">

<sub>🇫🇷 <a href="README.md">Français</a> · <a href="README.en.md">🇬🇧 English</a> · <a href="README.de.md">🇩🇪 Deutsch</a> · <a href="README.es.md">🇪🇸 Español</a> · BookScan — <a href="https://atelier.atechnologie.fr/">l'Atelier</a></sub>

</div>
