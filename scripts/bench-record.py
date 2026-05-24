#!/usr/bin/env python3
"""
Appends a benchmark entry to the NDJSON history and regenerates the Markdown table.

Usage:
    bench-record.py <bench_json_file> \\
        --tree=<git_tree_hash> --branch=<name> --date=<iso8601> --build-ms=<ms> \\
        --ndjson=<path> --markdown=<path>
"""

import argparse
import json
import sys
from pathlib import Path

PERF_COLS = [
    ("build_ms", "Build (ms)"),
    ("shader_validation_us", "Shader valid. (µs)"),
    ("sphere_128_64_us", "Sphere 128×64 (µs)"),
    ("scene_update_us", "Scene upd. (µs)"),
]
REGRESSION_THRESHOLD_PCT = 10


def delta_str(prev: float, curr: float) -> str:
    if prev == 0:
        return ""
    pct = (curr - prev) / prev * 100
    sign = "+" if pct >= 0 else ""
    flag = " ⚠️" if pct > REGRESSION_THRESHOLD_PCT else ""
    return f" ({sign}{pct:.0f}%){flag}"


def render_markdown(entries: list) -> str:
    headers = ["Tree", "Date", "Branch"] + [label for _, label in PERF_COLS]
    sep = [":---"] * len(headers)

    lines = [
        "# Performance History\n",
        "| " + " | ".join(headers) + " |",
        "| " + " | ".join(sep) + " |",
    ]

    for i, entry in enumerate(entries):
        prev = entries[i - 1] if i > 0 else None
        tree = entry.get("tree", "")[:8]
        date = entry.get("date", "")[:10]
        branch = entry.get("branch", "")

        metric_cells = []
        for key, _ in PERF_COLS:
            val = entry.get(key)
            if val is None:
                metric_cells.append("—")
            else:
                cell = str(val)
                if prev is not None and prev.get(key) is not None:
                    cell += delta_str(float(prev[key]), float(val))
                metric_cells.append(cell)

        row = [tree, date, branch] + metric_cells
        lines.append("| " + " | ".join(row) + " |")

    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("bench_json_file", help="Path to bench binary JSON output")
    parser.add_argument("--tree", required=True)
    parser.add_argument("--branch", required=True)
    parser.add_argument("--date", required=True)
    parser.add_argument("--build-ms", type=int, required=True)
    parser.add_argument("--ndjson", required=True)
    parser.add_argument("--markdown", required=True)
    args = parser.parse_args()

    with open(args.bench_json_file) as f:
        data = json.load(f)

    data.update(
        {
            "tree": args.tree,
            "date": args.date,
            "branch": args.branch,
            "build_ms": args.build_ms,
        }
    )

    ndjson_path = Path(args.ndjson)
    with open(ndjson_path, "a") as f:
        f.write(json.dumps(data) + "\n")

    entries = []
    with open(ndjson_path) as f:
        for lineno, line in enumerate(f, start=1):
            line = line.strip()
            if line:
                try:
                    entries.append(json.loads(line))
                except json.JSONDecodeError as e:
                    print(f"[perf] Warning: skipping malformed line {lineno} in {ndjson_path}: {e}", file=sys.stderr)

    Path(args.markdown).write_text(render_markdown(entries))
    print(f"[perf] {len(entries)} entries in history")


if __name__ == "__main__":
    main()
