#!/usr/bin/env python3
import csv
import os
from dataclasses import dataclass


SRC = os.path.join("profiling", "perf_profile.csv")
OUT = os.path.join("profiling", "perf_baseline.csv")
RUN_GAP_MS = 3000


@dataclass
class Row:
	unix_ms: int
	full_ms: float
	lighting_ms: float
	shadow_ms: float
	displacement_ms: float


def load_rows(path: str) -> list[Row]:
	if not os.path.exists(path):
		return []
	rows: list[Row] = []
	with open(path, "r", newline="", encoding="utf-8") as f:
		reader = csv.DictReader(f)
		for r in reader:
			try:
				rows.append(
					Row(
						unix_ms=int(float(r["unix_ms"])),
						full_ms=float(r["full_ms"]),
						lighting_ms=float(r["lighting_ms"]),
						shadow_ms=float(r["shadow_ms"]),
						displacement_ms=float(r["displacement_ms"]),
					)
				)
			except Exception:
				pass
	rows.sort(key=lambda x: x.unix_ms)
	return rows


def split_runs(rows: list[Row]) -> list[list[Row]]:
	if not rows:
		return []
	runs: list[list[Row]] = [[rows[0]]]
	for row in rows[1:]:
		if row.unix_ms - runs[-1][-1].unix_ms > RUN_GAP_MS:
			runs.append([row])
		else:
			runs[-1].append(row)
	return runs


def summarize(run: list[Row]) -> dict:
	n = max(len(run), 1)
	full = sum(r.full_ms for r in run) / n
	light = sum(r.lighting_ms for r in run) / n
	shadow = sum(r.shadow_ms for r in run) / n
	disp = sum(r.displacement_ms for r in run) / n
	avg_step = (light + shadow + disp) / 3.0
	return {
		"start_ms": run[0].unix_ms,
		"end_ms": run[-1].unix_ms,
		"samples": len(run),
		"full_ms": full,
		"lighting_ms": light,
		"shadow_ms": shadow,
		"displacement_ms": disp,
		"avg_step_ms": avg_step,
	}


def read_previous_baseline(path: str) -> dict:
	if not os.path.exists(path):
		return {}
	with open(path, "r", newline="", encoding="utf-8") as f:
		reader = csv.DictReader(f)
		for r in reader:
			if r.get("slot") == "latest":
				return r
	return {}


def fmt(v: float) -> str:
	return f"{v:.4f}"


def write_baseline(path: str, latest: dict, previous: dict | None) -> None:
	os.makedirs(os.path.dirname(path), exist_ok=True)
	fields = [
		"slot",
		"source",
		"start_ms",
		"end_ms",
		"samples",
		"full_ms",
		"lighting_ms",
		"shadow_ms",
		"displacement_ms",
		"avg_step_ms",
	]
	with open(path, "w", newline="", encoding="utf-8") as f:
		w = csv.DictWriter(f, fieldnames=fields)
		w.writeheader()
		w.writerow(
			{
				"slot": "latest",
				"source": "perf_profile",
				"start_ms": str(latest["start_ms"]),
				"end_ms": str(latest["end_ms"]),
				"samples": str(latest["samples"]),
				"full_ms": fmt(latest["full_ms"]),
				"lighting_ms": fmt(latest["lighting_ms"]),
				"shadow_ms": fmt(latest["shadow_ms"]),
				"displacement_ms": fmt(latest["displacement_ms"]),
				"avg_step_ms": fmt(latest["avg_step_ms"]),
			}
		)
		if previous:
			w.writerow(previous)


def to_previous_row(src: dict) -> dict:
	return {
		"slot": "previous",
		"source": src.get("source", "perf_profile"),
		"start_ms": str(src.get("start_ms", "0")),
		"end_ms": str(src.get("end_ms", "0")),
		"samples": str(src.get("samples", "0")),
		"full_ms": str(src.get("full_ms", "0")),
		"lighting_ms": str(src.get("lighting_ms", "0")),
		"shadow_ms": str(src.get("shadow_ms", "0")),
		"displacement_ms": str(src.get("displacement_ms", "0")),
		"avg_step_ms": str(src.get("avg_step_ms", "0")),
	}


def empty_previous_row() -> dict:
	return {
		"slot": "previous",
		"source": "none",
		"start_ms": "0",
		"end_ms": "0",
		"samples": "0",
		"full_ms": "0.0000",
		"lighting_ms": "0.0000",
		"shadow_ms": "0.0000",
		"displacement_ms": "0.0000",
		"avg_step_ms": "0.0000",
	}


def main() -> int:
	rows = load_rows(SRC)
	if not rows:
		print(f"No data in {SRC}")
		return 1
	runs = split_runs(rows)
	latest = summarize(runs[-1])
	previous = summarize(runs[-2]) if len(runs) >= 2 else None
	if previous is None:
		prev_latest = read_previous_baseline(OUT)
		if prev_latest:
			previous = to_previous_row(prev_latest)
	if previous is None:
		previous = empty_previous_row()
	write_baseline(OUT, latest, to_previous_row(previous) if previous and "slot" not in previous else previous)
	print(f"Wrote {OUT}")
	return 0


if __name__ == "__main__":
	raise SystemExit(main())
