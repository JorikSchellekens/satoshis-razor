#!/usr/bin/env python3
"""Snapshot the 1000+ theorems list into registry proposals.

The 1000+ project (https://1000-plus.github.io/, the successor of Freek
Wiedijk's "Formalizing 100 Theorems") catalogues notable theorems and
records which proof assistants have formalized each one. This script takes
the theorems that have NO Lean proof yet and writes them as a proposals
snapshot - one JSON object per line, the input format of
`razor propose-batch`.

The snapshot is committed to the repo (ingest/data/thousand-plus.jsonl) so
seeding the registry needs no network and is reproducible; this script is
how the snapshot is refreshed.

Usage:
  uv run ingest/fetch_thousand_plus.py --as-of 2026-07-03
  uv run ingest/fetch_thousand_plus.py --as-of 2026-07-03 --from-dir <checkout>
"""

import argparse
import html
import io
import json
import os
import re
import sys
import tarfile
import urllib.request

TARBALL = "https://github.com/1000-plus/1000-plus.github.io/archive/refs/heads/main.tar.gz"
OUT = os.path.join(os.path.dirname(__file__), "data", "thousand-plus.jsonl")


def parse_entry(stem, text):
    """Parse one _thm/*.md file: frontmatter-ish YAML the project uses.

    `stem` is the filename without .md - the catalogue's own unique key
    (usually the wikidata id, with a suffix when several theorems share
    one wikidata entry, e.g. Q657469X)."""
    title = None
    m = re.search(r"^# (.+)$", text, re.M)
    if m:
        # Some catalogue titles carry raw HTML entities ("Stolper&ndash;Samuelson").
        title = html.unescape(m.group(1).strip())
    wikidata = None
    m = re.search(r"^wikidata: (\S+)", text, re.M)
    if m:
        wikidata = m.group(1).strip()
    msc = None
    m = re.search(r"^msc_classification: [\"']?([^\"'\n]+)[\"']?", text, re.M)
    if m:
        msc = m.group(1).strip()
    wiki = None
    m = re.search(r"^\s*- [\"']\[\[([^\]|]+)", text, re.M)
    if m:
        wiki = m.group(1).strip()
    lean_status = None
    m = re.search(r"^lean:\n- status: (\w+)", text, re.M)
    if m:
        lean_status = m.group(1)
    elif "lean:" in text:
        lean_status = "unknown"
    return stem, title, wikidata, msc, wiki, lean_status


def entries(from_dir):
    if from_dir:
        thm = os.path.join(from_dir, "_thm")
        for name in sorted(os.listdir(thm)):
            if name.endswith(".md"):
                with open(os.path.join(thm, name)) as f:
                    yield parse_entry(name[:-3], f.read())
    else:
        print(f"fetching {TARBALL} ...", file=sys.stderr)
        data = urllib.request.urlopen(TARBALL).read()
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as tar:
            for member in sorted(tar.getmembers(), key=lambda m: m.name):
                if "/_thm/" in member.name and member.name.endswith(".md"):
                    stem = os.path.basename(member.name)[:-3]
                    yield parse_entry(stem, tar.extractfile(member).read().decode())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--as-of", required=True, help="date the list was read (YYYY-MM-DD)")
    ap.add_argument("--from-dir", help="parse an existing checkout instead of fetching")
    args = ap.parse_args()

    total = formalized = 0
    rows = []
    for stem, title, wikidata, msc, wiki, lean_status in entries(args.from_dir):
        total += 1
        if lean_status == "formalized":
            formalized += 1
            continue
        if not (title and stem):
            continue
        status_note = (
            "a Lean statement exists but no proof"
            if lean_status == "statement"
            else "no Lean formalization"
        )
        body_bits = [
            f"From the 1000+ theorems list, a community catalogue of notable "
            f"theorems and which proof assistants have formalized them. "
            f"As of {args.as_of} this entry records {status_note}.",
        ]
        if msc:
            body_bits.append(f"Mathematics Subject Classification: {msc}.")
        if wiki:
            body_bits.append(
                f"Background: https://en.wikipedia.org/wiki/{wiki.replace(' ', '_')}."
            )
        body_bits.append(
            f"Source: https://1000-plus.github.io/ (entry {stem}). "
            f"A sorry under this proposal should pin a Lean statement in the "
            f"Mathlib environment (razor sorry --env mathlib), using "
            f"Mathlib's definitions so an admitted proof is ready to build on."
        )
        rows.append({
            "id": f"THM-{stem}",
            "title": title,
            "body": " ".join(body_bits),
        })

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w") as f:
        for row in rows:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")
    print(
        f"{total} theorems catalogued; {formalized} already have Lean proofs; "
        f"{len(rows)} written to {OUT} as open proposals (as of {args.as_of})",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
