"""Launch MedQuiz (Rust / Dioxus)."""
import subprocess
import sys
from pathlib import Path

root = Path(__file__).resolve().parent
raise SystemExit(subprocess.call(["cargo", "run"], cwd=root))