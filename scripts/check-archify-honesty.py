#!/usr/bin/env python3
"""Archify honesty guard.

Archify 3.x ignores `meta.views` (guided chapters): anything authored there is
silently dropped from the rendered diagram. Honesty / status wording ("Probed",
"Simulated", "Not hosted", "Not FEAT_NMI", ...) that lives only in those retired
chapter notes therefore vanishes from the page. This guard fails when that can
happen. Standard library only (no installs).

For every committed Archify IR (a *.json with `schema_version`, `diagram_type`
and `meta.output`) it
  1. FAILS if `meta.views` exists (retired in 3.x; content silently dropped);
  2. FAILS if a configured honesty label/phrase appears in a retired view/chapter
     note but nowhere in the rendered text (component/participant labels,
     sublabels, tags, card titles/items, ...);
  3. WARNS (non-failing) if the committed HTML's generator string is not the
     expected one (default `archify 3.2.0`), or the HTML is missing.

Usage:  python3 scripts/check-archify-honesty.py [--root DIR] [--config FILE]
Config: scripts/archify-honesty.config.json
"""

from __future__ import annotations

import argparse
import html as html_lib
import json
import os
import posixpath
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
DEFAULT_CONFIG_PATH = HERE / "archify-honesty.config.json"

DEFAULTS = {
    "labels": ["Probed", "Simulated", "Not hosted", "Not Live", "Proposed", "Unknown", "Documented", "Gap", "Verified"],
    "phrases": [],
    "expectedGenerator": "archify 3.2.0",
    "generatorExemptTypes": ["timeline"],  # hand-rendered by chronify; no archify generator meta
    "html": {},  # IR path -> committed HTML path (repo-relative), when not meta.output beside the IR
    "ignore": [],  # repo-relative path prefixes to skip
    "renderedKeys": [
        "label", "sublabel", "tag", "tags", "title", "subtitle", "items",
        "detail", "note", "text", "description", "honesty",
    ],
    "caseSensitive": True,
}

RETIRED_KEYS = ("views", "chapters")  # top-level or under meta
SKIP_DIRS = {"node_modules", ".git", "target", "dist", "build", "coverage", ".venv", "__pycache__"}


def load_config(path=None) -> dict:
    user = {}
    if path and Path(path).exists():
        user = json.loads(Path(path).read_text(encoding="utf-8"))
    cfg = {**DEFAULTS, **user}
    for key in ("labels", "phrases", "generatorExemptTypes", "ignore", "renderedKeys"):
        if not isinstance(cfg[key], list) or not all(isinstance(v, str) for v in cfg[key]):
            raise ValueError(f"config.{key} must be an array of strings")
    return cfg


def norm(s) -> str:
    return re.sub(r"\s+", " ", str(s)).strip()


def matcher(term: str, case_sensitive: bool = True):
    return re.compile(
        r"(?<![A-Za-z0-9_])" + re.escape(norm(term)) + r"(?![A-Za-z0-9_])",
        0 if case_sensitive else re.IGNORECASE,
    )


def _collect(node, keys, out, skip_retired):
    """Collect strings stored under any of `keys` (string or list of strings), recursively."""
    if isinstance(node, list):
        for v in node:
            _collect(v, keys, out, skip_retired)
    elif isinstance(node, dict):
        for k, v in node.items():
            if skip_retired and k in RETIRED_KEYS:
                continue
            if k in keys:
                if isinstance(v, str):
                    out.append(v)
                elif isinstance(v, list):
                    out.extend(x for x in v if isinstance(x, str))
            if isinstance(v, (dict, list)):
                _collect(v, keys, out, skip_retired)


def rendered_text(ir: dict, cfg: dict) -> str:
    out: list[str] = []
    _collect(ir, set(cfg["renderedKeys"]), out, True)
    return norm(" \n ".join(out))


def retired_notes(ir: dict) -> list[dict]:
    notes = []
    meta = ir.get("meta") if isinstance(ir, dict) else None
    for prefix, holder in (("", ir), ("meta.", meta)):
        if not isinstance(holder, dict):
            continue
        for key in RETIRED_KEYS:
            items = holder.get(key)
            if not isinstance(items, list):
                continue
            for i, v in enumerate(items):
                ident = v["id"] if isinstance(v, dict) and v.get("id") else str(i)
                texts: list[str] = []
                if isinstance(v, str):
                    texts.append(v)
                else:
                    _collect(v, {"label", "title", "note", "text", "detail", "description", "items"}, texts, False)
                notes.extend({"where": f"{prefix}{key}[{ident}]", "text": norm(t)} for t in texts)
    return notes


def is_archify_ir(obj) -> bool:
    return (
        isinstance(obj, dict)
        and "schema_version" in obj
        and isinstance(obj.get("diagram_type"), str)
        and isinstance(obj.get("meta"), dict)
        and isinstance(obj["meta"].get("output"), str)
    )


def _decode_entities(s: str) -> str:
    return html_lib.unescape(s)


def generators_of(html: str) -> list[str]:
    """Every `<meta name="generator">` content in the page, including pages embedded entity-escaped (iframe srcdoc)."""
    found: list[str] = []
    for text in (html, _decode_entities(html)):
        for tag in re.findall(r"<meta\b[^>]*>", text, flags=re.IGNORECASE):
            if not re.search(r"\bname\s*=\s*[\"']generator[\"']", tag, flags=re.IGNORECASE):
                continue
            m = re.search(r"\bcontent\s*=\s*\"([^\"]*)\"|\bcontent\s*=\s*'([^']*)'", tag, flags=re.IGNORECASE)
            found.append((m.group(1) if m.group(1) is not None else m.group(2)) if m else "")
        if found:
            break  # literal tags win; fall back to the decoded text only if there are none
    return found


_UNSET = object()


def check_ir(ir: dict, cfg: dict, file: str = "<ir>", html=_UNSET) -> dict:
    """Check one parsed IR. `html` is the committed HTML text, None (missing) or omitted (skip)."""
    errors: list[str] = []
    warnings: list[str] = []
    meta = ir.get("meta") or {}

    if "views" in meta:
        errors.append(
            f"{file}: meta.views exists. Archify 3.x ignores it, so its chapter notes are silently dropped from "
            "the rendered diagram. Move any wording you still need into components (label/sublabel/tag) or "
            "cards, then delete meta.views."
        )

    rendered = rendered_text(ir, cfg)
    terms = list(dict.fromkeys([*cfg["labels"], *cfg["phrases"]]))
    for note in retired_notes(ir):
        for term in terms:
            rx = matcher(term, cfg["caseSensitive"])
            if rx.search(note["text"]) and not rx.search(rendered):
                errors.append(
                    f'{file}: honesty wording "{term}" appears in retired {note["where"]} '
                    f'("{note["text"][:80]}") but not in any component label/sublabel/tag or card item, '
                    "so it is not rendered."
                )

    if html is not _UNSET and ir.get("diagram_type") not in cfg["generatorExemptTypes"]:
        if html is None:
            warnings.append(
                f'{file}: committed HTML for meta.output "{meta.get("output")}" not found; cannot check its '
                'generator string (set "html" in the config if it is rendered elsewhere).'
            )
        else:
            gens = generators_of(html)
            expected = cfg["expectedGenerator"]
            bad = list(dict.fromkeys(g for g in gens if g != expected))
            if not gens:
                warnings.append(f'{file}: committed HTML has no <meta name="generator"> (expected "{expected}").')
            elif bad:
                found = '", "'.join(bad)
                warnings.append(
                    f'{file}: committed HTML generator is "{found}", expected "{expected}". '
                    "Regenerate with the pinned Archify."
                )
    return {"errors": errors, "warnings": warnings}


def list_candidate_files(root: Path) -> list[str]:
    try:
        out = subprocess.run(
            ["git", "-C", str(root), "ls-files", "-z", "--", "*.json"],
            capture_output=True, text=True, check=True,
        ).stdout
        files = [f for f in out.split("\0") if f and (root / f).exists()]
        if files:
            return files
    except (OSError, subprocess.CalledProcessError):
        pass  # not a git checkout: fall through to a directory walk
    found = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        found.extend((Path(dirpath) / n).relative_to(root).as_posix() for n in filenames if n.endswith(".json"))
    return found


def run(root: Path, cfg: dict) -> dict:
    root = Path(root)
    errors: list[str] = []
    warnings: list[str] = []
    checked = 0
    for rel in sorted(list_candidate_files(root)):
        if any(rel == p or rel.startswith(p if p.endswith("/") else p + "/") for p in cfg["ignore"]):
            continue
        try:
            ir = json.loads((root / rel).read_text(encoding="utf-8"))
        except (OSError, ValueError):
            continue
        if not is_archify_ir(ir):
            continue
        checked += 1
        html_rel = cfg["html"].get(rel) or posixpath.join(posixpath.dirname(rel), ir["meta"]["output"])
        html_path = root / html_rel
        html = html_path.read_text(encoding="utf-8", errors="replace") if html_path.exists() else None
        result = check_ir(ir, cfg, file=rel, html=html)
        errors.extend(result["errors"])
        warnings.extend(result["warnings"])
    return {"checked": checked, "errors": errors, "warnings": warnings}


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="Fail when honesty wording lives only in retired Archify chapter notes.")
    ap.add_argument("--root", default=os.getcwd())
    ap.add_argument("--config", default=str(DEFAULT_CONFIG_PATH))
    args = ap.parse_args(argv)
    try:
        cfg = load_config(args.config)
        result = run(Path(args.root).resolve(), cfg)
    except Exception as exc:  # fail closed on a broken config
        print(f"✗ {exc}", file=sys.stderr)
        return 1
    gha = os.environ.get("GITHUB_ACTIONS") == "true"
    for w in result["warnings"]:
        print(f"::warning::{w}" if gha else f"⚠ {w}", file=sys.stderr)
    for e in result["errors"]:
        print(f"::error::{e}" if gha else f"✗ {e}", file=sys.stderr)
    if not result["checked"]:
        print("⚠ no committed Archify IR found; nothing to check", file=sys.stderr)
    if result["errors"]:
        return 1
    print(f"✓ archify honesty guard: {result['checked']} diagram(s) checked, {len(result['warnings'])} warning(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
