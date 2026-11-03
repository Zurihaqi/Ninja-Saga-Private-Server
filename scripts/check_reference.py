"""Syntax-check first-party reference source without executing application code.

Exit 0: all checks passed. Exit 1: syntax failure. Exit 2: tool/check failure.
"""
from concurrent.futures import ThreadPoolExecutor
import hashlib
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "reference/upstream/Database/Laragon/ninjasage"
REPORT = ROOT / "docs/baseline-static-checks.md"
COMMIT = "ecf9012d885d1c89fd5f42791eed4f37f90f2edd"


def run(command):
    return subprocess.run(command, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=30)


def check(task):
    language, executable, path = task
    args = [executable, "-l" if language == "PHP" else "--check", str(path)]
    try:
        result = run(args)
    except (OSError, subprocess.TimeoutExpired) as error:
        return language, path, "error", type(error).__name__
    if result.returncode == 0:
        return language, path, "pass", ""
    # Avoid copying arbitrary source excerpts from Node's error output.
    lines = (result.stderr + "\n" + result.stdout).splitlines()
    errors = [line.strip() for line in lines if any(marker in line for marker in ("Parse error:", "Fatal error:", "SyntaxError:", "Error:"))]
    summary = " / ".join(dict.fromkeys(errors)) or f"checker exited {result.returncode}"
    summary = summary.replace(str(SOURCE), "<reference>")
    return language, path, "fail", summary


def main():
    php, node = shutil.which("php"), shutil.which("node")
    if not php or not node:
        raise SystemExit("Both php and node must be available on PATH (no application dependencies are required).")
    php_files = sorted(p for directory in ("app", "routes", "database", "tests") for p in (SOURCE / directory).rglob("*.php") if "ThirdParty" not in p.parts)
    js_files = sorted(p for directory in ("chat-server", "pvp-server") for p in (SOURCE / directory).rglob("*.js") if "node_modules" not in p.parts)
    if not php_files or not js_files:
        raise SystemExit("Reference source missing or incomplete.")
    tasks = [("PHP", php, p) for p in php_files] + [("JavaScript", node, p) for p in js_files]
    with ThreadPoolExecutor(max_workers=4) as executor:
        results = list(executor.map(check, tasks))
    versions = [run([php, "--version"]).stdout.splitlines()[0], run([node, "--version"]).stdout.strip()]
    digest = hashlib.sha256()
    for _, _, p in tasks:
        digest.update(p.relative_to(SOURCE).as_posix().encode())
        digest.update(b"\x00")
        digest.update(p.read_bytes())
        digest.update(b"\x00")
    report = "# Reference static validation\n\n"
    report += f"Pinned source: `{COMMIT}`. Regenerate from the workspace root with `python scripts/check_reference.py`.\n\n"
    report += "Only parser checks were run: `php -l` for first-party app/routes/database/tests PHP (excluding app/ThirdParty), and `node --check` for chat/PvP JavaScript (excluding node_modules). Neither command executes application code. No database connection, seeder, dependency install, HTTP request or game client was used.\n\n"
    report += "Tools: " + "; ".join(f"`{version}`" for version in versions) + ".\n\n"
    report += f"Ordered path/content SHA-256 for checked inputs: `{digest.hexdigest()}`.\n\n"
    report += "| Language | Files | Passed | Syntax failures | Check errors |\n|---|---:|---:|---:|---:|\n"
    for language in ("PHP", "JavaScript"):
        group = [r for r in results if r[0] == language]
        counts = {state: sum(r[2] == state for r in group) for state in ("pass", "fail", "error")}
        report += f"| {language} | {len(group)} | {counts['pass']} | {counts['fail']} | {counts['error']} |\n"
    failures = [r for r in results if r[2] != "pass"]
    report += "\n## Failures\n\n"
    if not failures:
        report += "None.\n"
    else:
        for language, path, state, error in failures:
            report += f"- **{state} / {language}** `{path.relative_to(SOURCE).as_posix()}`: {error}\n"
    report += "\n## Limits\n\nPassing syntax checks does not establish dependency compatibility, Laravel bootability, migration correctness, authorization, game behavior or client/protocol compatibility. PHP runtime version may differ from the upstream minimum. Full baseline execution remains a separate acceptance gate.\n"
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(report, encoding="utf-8")
    print(f"Checked {len(results)} files: {len(failures)} failures/errors. Report: {REPORT.relative_to(ROOT)}")
    return 2 if any(r[2] == "error" for r in results) else 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
