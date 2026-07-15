#!/usr/bin/env python3
"""Fail when repository Markdown contains an absolute local or broken relative link."""

import argparse
import re
import sys
from pathlib import Path
from typing import Iterable, List, Tuple
from urllib.parse import unquote


INLINE_LINK = re.compile(r"!?(?:\[[^\]]*\])\(([^)]+)\)")
REFERENCE_LINK = re.compile(r"^\s*\[[^\]]+\]:\s*(\S+)", re.MULTILINE)
LOCAL_ABSOLUTE = re.compile(r"^(?:file://|/(?:Users|home|private|tmp)/)")
SKIP_PREFIXES = ("http://", "https://", "mailto:", "#", "data:")


def markdown_files(root: Path) -> Iterable[Path]:
    for path in sorted(root.rglob("*.md")):
        if not any(part in {".git", "target"} for part in path.parts):
            yield path


def destinations(text: str) -> Iterable[str]:
    for pattern in (INLINE_LINK, REFERENCE_LINK):
        for match in pattern.finditer(text):
            destination = match.group(1).strip().strip("<>")
            if " " in destination and not destination.startswith(
                ("http://", "https://")
            ):
                destination = destination.split(" ", 1)[0]
            yield destination


def check(root: Path) -> List[Tuple[Path, str, str]]:
    failures: List[Tuple[Path, str, str]] = []
    for document in markdown_files(root):
        for destination in destinations(document.read_text(errors="replace")):
            if destination.startswith(SKIP_PREFIXES):
                continue
            if LOCAL_ABSOLUTE.match(destination):
                failures.append((document, destination, "absolute local link"))
                continue
            relative = unquote(destination.split("#", 1)[0])
            if not relative:
                continue
            target = (document.parent / relative).resolve()
            try:
                target.relative_to(root.resolve())
            except ValueError:
                failures.append((document, destination, "link escapes repository"))
                continue
            if not target.exists():
                failures.append((document, destination, "target does not exist"))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("root", nargs="?", type=Path, default=Path.cwd())
    args = parser.parse_args()
    failures = check(args.root)
    for document, destination, reason in failures:
        print(f"{document}: {destination}: {reason}", file=sys.stderr)
    if failures:
        return 1
    print("all repository-relative Markdown links resolve")
    return 0


if __name__ == "__main__":
    sys.exit(main())
