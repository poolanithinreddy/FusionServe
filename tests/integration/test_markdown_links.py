import importlib.util
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/check_markdown_links.py"
SPEC = importlib.util.spec_from_file_location("check_markdown_links", SCRIPT)
assert SPEC and SPEC.loader
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)


def test_accepts_valid_relative_and_external_links(tmp_path):
    (tmp_path / "target.md").write_text("# Target\n")
    (tmp_path / "source.md").write_text(
        "[relative](target.md) [anchor](#section) [external](https://example.com)\n"
    )
    assert CHECKER.check(tmp_path) == []


def test_rejects_broken_and_absolute_local_links(tmp_path):
    source = tmp_path / "source.md"
    source.write_text("[broken](missing.md) [private](file:///tmp/private.md)\n")
    failures = CHECKER.check(tmp_path)
    assert [(destination, reason) for _, destination, reason in failures] == [
        ("missing.md", "target does not exist"),
        ("file:///tmp/private.md", "absolute local link"),
    ]
