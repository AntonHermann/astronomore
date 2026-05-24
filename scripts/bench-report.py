#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "marimo>=0.9",
#   "polars>=1.0",
#   "altair>=5.3",
#   "vl-convert-python>=1.3",
# ]
# ///
import marimo

__generated_with = "0.9.0"
app = marimo.App(width="medium", title="Bench Range Report")


@app.cell
def _():
    import marimo as mo
    import polars as pl
    import altair as alt
    from pathlib import Path
    return Path, alt, mo, pl


@app.cell
def _(mo):
    file_path_widget = mo.ui.text(
        value="perf/range-results.ndjson",
        label="NDJSON file (range-results.ndjson or benchmarks.ndjson)",
        full_width=True,
    )
    file_path_widget
    return (file_path_widget,)


@app.cell
def _(Path, file_path_widget, mo, pl):
    _p = Path(file_path_widget.value)
    mo.stop(
        not _p.exists(),
        mo.callout(
            mo.md(
                f"**File not found:** `{_p}`\n\n"
                "Run `just bench-range` first, then refresh."
            ),
            kind="warn",
        ),
    )
    _raw = pl.read_ndjson(_p)

    # Unified label column — prefers 'sha' (range output) over 'tree' (hook output)
    _id_col = "sha" if "sha" in _raw.columns else "tree"

    # Ensure boolean sentinel columns exist (hook-generated NDJSON omits them)
    _extra = []
    for _col in ("bench_unavailable", "gpu_skipped"):
        if _col in _raw.columns:
            _extra.append(pl.col(_col).fill_null(False))
        else:
            _extra.append(pl.lit(False).alias(_col))

    df = _raw.with_columns([pl.col(_id_col).str.slice(0, 8).alias("label"), *_extra])
    df
    return (df,)


@app.cell
def _(df, mo, pl):
    _want = ["label", "subject", "date", "build_ms",
             "shader_validation_us", "sphere_128_64_us", "scene_update_us"]
    _cols = [c for c in _want if c in df.columns]
    mo.ui.table(
        df.select([pl.col(c) for c in _cols]).to_dicts(),
        label="Benchmark history",
    )
    return ()


@app.cell
def _(alt, df):
    build_chart = (
        alt.Chart(df)
        .mark_line(point=True)
        .encode(
            x=alt.X("label:N", title="Commit", sort=None),
            y=alt.Y("build_ms:Q", title="Build time (ms)"),
            color=alt.Color(
                "bench_unavailable:N",
                title="No bench binary",
                scale=alt.Scale(range=["steelblue", "tomato"]),
            ),
            tooltip=["label", "build_ms", "bench_unavailable"],
        )
        .properties(title="Build Time per Commit", width=700, height=280)
    )
    build_chart
    return (build_chart,)


@app.cell
def _(alt, df, mo, pl):
    if "shader_validation_us" not in df.columns:
        shader_chart = None
        _out = mo.md("_No `shader_validation_us` data in this file._")
    else:
        _df_s = df.filter(~pl.col("bench_unavailable"))
        shader_chart = (
            alt.Chart(_df_s)
            .mark_line(point=True)
            .encode(
                x=alt.X("label:N", title="Commit", sort=None),
                y=alt.Y("shader_validation_us:Q", title="Shader validation (µs)"),
                tooltip=["label", "shader_validation_us"],
            )
            .properties(title="Shader Validation Time per Commit", width=700, height=280)
        )
        _out = shader_chart
    _out
    return (shader_chart,)


@app.cell
def _(alt, df, mo, pl):
    _have = "sphere_128_64_us" in df.columns and "scene_update_us" in df.columns
    if not _have:
        gpu_chart = None
        _out = mo.md("_No GPU benchmark columns in this file._")
    else:
        _df_g = df.filter(~pl.col("bench_unavailable") & ~pl.col("gpu_skipped")).select(
            ["label", "sphere_128_64_us", "scene_update_us"]
        )
        if _df_g.is_empty():
            gpu_chart = None
            _out = mo.callout(
                mo.md("No GPU rows — all runs had `gpu_skipped=true`."), kind="info"
            )
        else:
            _df_long = _df_g.unpivot(
                index="label",
                on=["sphere_128_64_us", "scene_update_us"],
                variable_name="metric",
                value_name="us",
            )
            gpu_chart = (
                alt.Chart(_df_long)
                .mark_line(point=True)
                .encode(
                    x=alt.X("label:N", title="Commit", sort=None),
                    y=alt.Y("us:Q", title="Time (µs)"),
                    color=alt.Color("metric:N", title="Benchmark"),
                    tooltip=["label", "metric", "us"],
                )
                .properties(title="GPU Benchmarks per Commit", width=700, height=280)
            )
            _out = gpu_chart
    _out
    return (gpu_chart,)


@app.cell
def _(Path, build_chart, df, gpu_chart, pl, shader_chart):
    import vl_convert as vlc

    _plots_dir = Path("perf/plots")
    _plots_dir.mkdir(parents=True, exist_ok=True)

    _written: list[str] = []
    for _name, _chart in [
        ("build_time", build_chart),
        ("shader_valid", shader_chart),
        ("gpu_benchmarks", gpu_chart),
    ]:
        if _chart is None:
            continue
        _svg = vlc.vegalite_to_svg(_chart.to_dict())
        _path = _plots_dir / f"{_name}.svg"
        if isinstance(_svg, bytes):
            _path.write_bytes(_svg)
        else:
            _path.write_text(_svg)
        _written.append(_name)

    # Markdown report
    _want = ["label", "build_ms", "shader_validation_us", "sphere_128_64_us", "scene_update_us"]
    _cols = [c for c in _want if c in df.columns]
    _rows = df.select([pl.col(c) for c in _cols]).to_dicts()
    _header = " | ".join(_cols)
    _sep = " | ".join([":---"] * len(_cols))
    _body = "\n".join(
        "| " + " | ".join(str(r.get(c, "—")) for c in _cols) + " |" for r in _rows
    )
    _img_lines = "\n".join(
        f"![{n.replace('_', ' ').title()}](plots/{n}.svg)" for n in _written
    )
    Path("perf/bench-report.md").write_text(
        f"# Benchmark Range Report\n\n"
        f"Generated by `scripts/bench-report.py`.\n\n"
        f"## Summary\n\n"
        f"| {_header} |\n| {_sep} |\n{_body}\n\n"
        f"## Charts\n\n{_img_lines}\n"
    )

    export_status = f"Wrote {len(_written)} SVG(s) → `perf/plots/` and `perf/bench-report.md`"
    export_status
    return (export_status,)


@app.cell
def _(export_status, mo):
    mo.callout(mo.md(export_status), kind="success")
    return ()


if __name__ == "__main__":
    app.run()
