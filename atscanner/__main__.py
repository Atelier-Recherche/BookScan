"""Deprecated entrypoint. Use bookscan companion (Rust) instead."""

import sys


def main() -> int:
    print(
        "ATscanner Tkinter prototype is deprecated.\n"
        "Run the desktop app:\n"
        "  cd bookscan-companion && cargo run --release\n"
        "And install the Android app from bookscan/ (gradlew assembleDebug)."
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
