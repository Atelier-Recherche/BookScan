import io
import os
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
EXE = ROOT / "bookscan-companion" / "target" / "release" / "bookscan-companion.exe"
APK = ROOT / "bookscan" / "app" / "build" / "outputs" / "apk" / "debug" / "app-debug.apk"
LIB = ROOT / ".bookscan_test_lib"
PORT = "7842"


def main() -> int:
    print("exe", EXE.exists(), EXE)
    print("apk", APK.exists(), APK)
    if not EXE.exists():
        return 1

    env = os.environ.copy()
    env["BOOKSCAN_LIBRARY"] = str(LIB)
    env["BOOKSCAN_PORT"] = PORT
    proc = subprocess.Popen([str(EXE)], env=env, cwd=str(EXE.parent))
    try:
        status = None
        for _ in range(40):
            time.sleep(0.4)
            try:
                with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/v1/status", timeout=1) as r:
                    status = r.read().decode()
                    break
            except Exception:
                continue
        print("status", status)
        if status is None:
            return 2

        img = Image.new("RGB", (400, 600), (240, 240, 240))
        buf = io.BytesIO()
        img.save(buf, format="JPEG")
        data = buf.getvalue()
        boundary = "----bound"
        parts = []
        parts.append(f"--{boundary}\r\nContent-Disposition: form-data; name=\"page_id\"\r\n\r\nsmoke1\r\n".encode())
        parts.append(
            (
                f"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; "
                f"filename=\"smoke1.jpg\"\r\nContent-Type: image/jpeg\r\n\r\n"
            ).encode()
        )
        parts.append(data)
        parts.append(f"\r\n--{boundary}--\r\n".encode())
        body = b"".join(parts)
        req = urllib.request.Request(
            f"http://127.0.0.1:{PORT}/v1/pages",
            data=body,
            method="POST",
            headers={"Content-Type": f"multipart/form-data; boundary={boundary}"},
        )
        with urllib.request.urlopen(req, timeout=60) as r:
            print("upload", r.read().decode())

        for _ in range(20):
            time.sleep(0.5)
            with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/v1/pages/smoke1", timeout=5) as r:
                page = r.read().decode()
            print("page", page)
            if '"ready"' in page or '"error"' in page:
                break
        return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()


if __name__ == "__main__":
    raise SystemExit(main())
