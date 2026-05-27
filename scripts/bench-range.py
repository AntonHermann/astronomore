#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""
Walk a git commit range, build the project at each commit, record timing metrics.

For commits with a bench binary: records full CPU/GPU timing.
For commits without (or build failure): records build time only, bench_unavailable=true.

Results are appended to the output file immediately so partial runs survive Ctrl-C.

Usage:
    uv run scripts/bench-range.py [--from COMMIT] [--to HEAD] [--count N]
                                   [--output FILE] [--append] [--dry-run]
"""

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path


def git(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(["git"] + list(args), capture_output=True, text=True)


def get_commits(to_sha: str, count: int, from_sha: str | None) -> list[dict]:
    """Return commit list oldest-first, each with sha/subject/date keys."""
    sep = "\x1f"
    cmd = ["git", "log", f"--format=%H{sep}%s{sep}%aI"]
    if from_sha:
        cmd.append(f"{from_sha}..{to_sha}")
    else:
        cmd.append(to_sha)
    cmd += ["-n", str(count)]

    r = subprocess.run(cmd, capture_output=True, text=True, check=True)
    commits = []
    for line in r.stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        parts = line.split(sep, 2)
        if len(parts) < 3:
            continue
        commits.append({"sha": parts[0], "subject": parts[1], "date": parts[2]})
    commits.reverse()  # oldest → newest
    return commits


def is_dirty() -> bool:
    r1 = subprocess.run(["git", "diff", "--quiet"], capture_output=True)
    r2 = subprocess.run(["git", "diff", "--cached", "--quiet"], capture_output=True)
    return r1.returncode != 0 or r2.returncode != 0


def timed_cargo(*args: str, timeout: int = 360) -> tuple[bool, int]:
    """Run cargo; return (success, elapsed_ms)."""
    t0 = time.monotonic()
    r = subprocess.run(
        ["cargo"] + list(args),
        capture_output=True,
        text=True,
        timeout=timeout,
    )
    return r.returncode == 0, int((time.monotonic() - t0) * 1000)


def benchmark_at(commit: dict, repo_root: Path) -> dict:
    """Build and optionally benchmark at the current checked-out commit."""
    bench_bin = repo_root / "target" / "release" / "bench"

    ok, build_ms = timed_cargo("build", "--release", "--bin", "bench", "--quiet")
    bench_available = ok
    build_failed = False

    if not ok:
        # Fall back: build full crate to at least capture a build time
        ok2, build_ms2 = timed_cargo("build", "--release", "--quiet")
        build_ms = build_ms2
        build_failed = not ok2

    record: dict = {
        "sha": commit["sha"],
        "subject": commit["subject"],
        "date": commit["date"],
        "build_ms": build_ms,
        "bench_unavailable": not bench_available,
        "build_failed": build_failed,
    }

    if bench_available and bench_bin.exists():
        try:
            r = subprocess.run(
                [str(bench_bin)], capture_output=True, text=True, timeout=30
            )
            if r.returncode == 0:
                record.update(json.loads(r.stdout))
            else:
                record["bench_error"] = r.stderr.strip()[:200]
        except (json.JSONDecodeError, subprocess.TimeoutExpired, OSError) as e:
            record["bench_error"] = str(e)

    return record


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--from", dest="from_sha", metavar="COMMIT",
        help="Start commit (exclusive). Default: unlimited (use --count to cap).",
    )
    parser.add_argument(
        "--to", dest="to_sha", metavar="COMMIT", default="HEAD",
        help="End commit (inclusive). Default: HEAD",
    )
    parser.add_argument(
        "--count", "-n", type=int, default=10, metavar="N",
        help="Max commits to benchmark. Default: 10",
    )
    parser.add_argument(
        "--output", "-o", default="perf/range-results.ndjson", metavar="FILE",
        help="Output NDJSON file. Default: perf/range-results.ndjson",
    )
    parser.add_argument("--append", action="store_true", help="Append to existing file.")
    parser.add_argument("--dry-run", action="store_true", help="Print commit list, no builds.")
    args = parser.parse_args()

    commits = get_commits(args.to_sha, args.count, args.from_sha)
    if not commits:
        print("No commits found.", file=sys.stderr)
        sys.exit(1)

    prefix = "DRY RUN — " if args.dry_run else ""
    print(f"{prefix}Benchmarking {len(commits)} commits (oldest → newest):")
    for c in commits:
        print(f"  {c['sha'][:8]}  {c['date'][:10]}  {c['subject'][:60]}")

    if args.dry_run:
        return

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)

    repo_root = Path(
        subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            capture_output=True, text=True, check=True,
        ).stdout.strip()
    )

    ref_result = git("symbolic-ref", "--short", "HEAD")
    original_ref = (
        ref_result.stdout.strip()
        if ref_result.returncode == 0
        else git("rev-parse", "HEAD").stdout.strip()
    )

    stashed = False
    if is_dirty():
        r = subprocess.run(
            ["git", "stash", "push", "-m", "bench-range auto-stash"],
            capture_output=True, text=True,
        )
        stashed = r.returncode == 0 and "No local changes" not in r.stdout

    try:
        mode = "a" if args.append else "w"
        with open(output, mode) as out:
            for i, commit in enumerate(commits, 1):
                sha = commit["sha"]
                print(
                    f"\n[{i}/{len(commits)}] {sha[:8]}  {commit['date'][:10]}  {commit['subject'][:55]}",
                    flush=True,
                )
                subprocess.run(["git", "checkout", "--quiet", sha], check=True)

                record = benchmark_at(commit, repo_root)
                out.write(json.dumps(record) + "\n")
                out.flush()

                status = "FAILED" if record.get("build_failed") else "ok"
                bench_note = (
                    "  (no bench binary)"
                    if record.get("bench_unavailable")
                    else f"  shader={record.get('shader_validation_us', '?')}µs"
                )
                print(f"           build={record['build_ms']}ms  {status}{bench_note}", flush=True)
    finally:
        subprocess.run(["git", "checkout", "--quiet", original_ref])
        if stashed:
            subprocess.run(["git", "stash", "pop", "--quiet"])

    print(f"\nDone. Results written to {output} ({len(commits)} entries).")


if __name__ == "__main__":
    main()
