"""Compile guarded panel repairs into the ignored local client, then verify bytecode.

The pinned reference movie is never edited. Run prepare-local-client.py before
this helper when creating a fresh local client. Java and .tools/ffdec are used
only during preparation; neither is a game server dependency.
"""
from __future__ import annotations

import argparse
import importlib.util
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
DEFAULT = ROOT / ".tools/client/NinjaSage.swf"


def run(java: str, *arguments: str) -> None:
    result = subprocess.run(
        [java, "-jar", str(ROOT / ".tools/ffdec/ffdec.jar"), *map(str, arguments)],
        cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace",
        timeout=240,
    )
    if result.returncode:
        raise RuntimeError((result.stdout + result.stderr)[-12000:])


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, default=DEFAULT)
    parser.add_argument("--output", type=Path, default=DEFAULT)
    parser.add_argument("--check", action="store_true", help="Verify installed repairs without changing the movie")
    args = parser.parse_args()
    source = args.input.resolve(strict=True)
    target = args.output.resolve()
    # Refuse accidental edits to the original client or tracked runtime source.
    local_root = (ROOT / ".tools").resolve()
    if not target.is_relative_to(local_root):
        raise ValueError("Panel repair output must stay inside this workspace's ignored .tools directory")
    java = shutil.which("java")
    if not java:
        raise RuntimeError("Java is required for the local client preparation step")
    patches: dict = {}
    verifications: dict = {}
    for helper in sorted((ROOT / "scripts/client-patches").glob("*.py")):
        spec = importlib.util.spec_from_file_location("panel_patch_" + helper.stem, helper)
        module = importlib.util.module_from_spec(spec)
        assert spec.loader is not None
        spec.loader.exec_module(module)
        for name, patch in module.PATCHES.items():
            if name in patches:
                raise ValueError(f"Two patch helpers claim {name}")
            patches[name] = patch
        verifications.update(getattr(module, "VERIFICATIONS", {}))
    if not patches:
        raise RuntimeError("No panel repair helpers were found")
    work = Path(tempfile.mkdtemp(prefix="client-panels-", dir=local_root))
    exported = work / "exported"
    classes = ",".join(patches)
    run(java, "-selectclass", classes, "-export", "script", exported, source)
    scripts = exported / "scripts"
    if args.check:
        verify(scripts, verifications)
        print(f"Verified {len(patches)} installed client class repairs.")
        return
    for name, patch in patches.items():
        script = scripts / Path(*name.split(".")).with_suffix(".as")
        original = script.read_text(encoding="utf-8-sig")
        repaired = patch(original)
        if not repaired.strip():
            raise ValueError(f"Empty patch output for {name}")
        script.write_text(repaired, encoding="utf-8")
    candidate = work / "NinjaSage.repaired.swf"
    run(java, "-air", "-onerror", "abort", "-importScript", source, candidate, scripts)
    verified = work / "verified"
    run(java, "-selectclass", classes, "-export", "script", verified, candidate)
    verify(verified / "scripts", verifications)
    backup = source.with_name(source.stem + ".before-panel-repairs.swf")
    if not backup.exists():
        shutil.copy2(source, backup)
    target.parent.mkdir(parents=True, exist_ok=True)
    staged = target.with_suffix(".repairing.swf")
    shutil.copy2(candidate, staged)
    staged.replace(target)
    print(f"Compiled and verified {len(patches)} client class repairs: {target}")
    print(f"Preparation evidence: {work}")


def verify(scripts: Path, verifications: dict) -> None:
    for name, markers in verifications.items():
        path = scripts / Path(*name.split(".")).with_suffix(".as")
        compiled_source = path.read_text(encoding="utf-8-sig")
        for marker in markers:
            if marker not in compiled_source:
                raise ValueError(f"Compiled repair verification failed: {name}: {marker}")


if __name__ == "__main__":
    main()
