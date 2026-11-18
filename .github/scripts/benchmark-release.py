import json
import os
import pathlib
import statistics
import subprocess
import sys
import time

root = pathlib.Path.cwd()
target = os.environ["BENCH_TARGET"]
profile = os.environ["BENCH_PROFILE"]
exe = root / "target" / target / "release" / ("tummler.exe" if sys.platform == "win32" else "tummler")
start = time.perf_counter()
subprocess.run(["cargo", "build", "--release", "--target", target], check=True)
build_seconds = time.perf_counter() - start
size = exe.stat().st_size
result = {
    "target": target,
    "profile": profile,
    "binary_bytes": size,
    "build_seconds": round(build_seconds, 2),
    "version_median_ms": None,
    "svg_convert_median_ms": None,
}

# Benchmark execution only on native targets; macOS x86_64 is cross-built from
# Apple Silicon in this matrix and cannot be run on the hosted arm64 runner.
if os.environ.get("BENCH_NATIVE") == "true":
    def median_ms(args):
        samples = []
        for _ in range(9):
            start = time.perf_counter()
            subprocess.run([str(exe), *args], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
            samples.append((time.perf_counter() - start) * 1000)
        return round(statistics.median(samples), 2)

    fixture = root / "benchmark-input.svg"
    fixture.write_text('<svg xmlns="http://www.w3.org/2000/svg" width="2048" height="2048"><rect width="2048" height="2048" fill="#2468ac"/><circle cx="1024" cy="1024" r="900" fill="#e5b840"/></svg>')
    result["version_median_ms"] = median_ms(["--version"])
    result["svg_convert_median_ms"] = median_ms([str(fixture), "png", "-o", str(root / "benchmark-output.png"), "--overwrite"])

path = pathlib.Path("benchmark.json")
path.write_text(json.dumps(result, indent=2) + "\n")
with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as f:
    f.write(f"| {target} | {profile} | {size} | {build_seconds:.2f}s | {result['version_median_ms']} | {result['svg_convert_median_ms']} |\n")
print(json.dumps(result))
