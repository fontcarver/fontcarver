#!/usr/bin/env python3
"""Audit installed, locked dependencies and prepare release attribution.

Run cargo fetch --locked and pnpm install --frozen-lockfile before this script.
No build, dependency installation, or network access is performed here.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
PROJECT_NOTICES = (
    "LICENSE-MIT", "LICENSE-APACHE", "THIRD_PARTY.md",
    "crates/fc-glyphdata/data/UNICODE-LICENSE.txt",
    "scripts/release_licenses/native/HOST-LIBRARIES.txt",
    "scripts/release_licenses/native/HOST-LGPL-2.txt",
    "scripts/release_licenses/native/HOST-LGPL-2.1.txt",
)
HOST_LICENSE_HASHES = {
    "scripts/release_licenses/native/HOST-LGPL-2.txt":
        "b7993225104d90ddd8024fd838faf300bea5e83d91203eab98e29512acebd69c",
    "scripts/release_licenses/native/HOST-LGPL-2.1.txt":
        "dc626520dcd53a22f727af3ee42c770e56c97a64fe3adb063799d8ab032fe551",
}
ALLOWED = {
    "MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "0BSD", "ISC",
    "Zlib", "BSL-1.0", "Unicode-3.0", "Unicode-DFS-2016", "CC0-1.0",
    "MIT-0", "Unlicense", "Apache-2.0 WITH LLVM-exception",
}
# These are existing rulings, not an extensible allow-list. Each must still be
# present in THIRD_PARTY.md, and a new version requires a new reviewed ruling.
MPL_RULINGS = {
    ("cssparser", "0.36.0"), ("cssparser-macros", "0.6.1"),
    ("dtoa-short", "0.3.5"), ("selectors", "0.36.1"), ("option-ext", "0.2.0"),
}
NPM_RULINGS = {
    ("caniuse-lite", "1.0.30001810"): "CC-BY-4.0",
    ("minimatch", "10.2.6"): "BlueOak-1.0.0",
}
NOTICE_NAME = re.compile(
    r"^(?:licen[cs]e|copying|copyright|notice|unlicense)(?:$|[._-])"
    r"|^third[._-]?party[._-]?notices?(?:text)?(?:$|[._-])", re.I,
)
LEGAL_SUFFIXES = {"", ".txt", ".md", ".rst", ".html", ".mit", ".apache"}


@dataclass(frozen=True)
class Dependency:
    """One installed version, with package-relative licensing metadata."""

    ecosystem: str
    name: str
    version: str
    license: str | None
    root: Path
    repository: str
    source: str = ""
    license_file: str | None = None


def allowed_expression(expression: str | None, exceptions: set[str]) -> bool:
    """Evaluate SPDX AND/OR with proper precedence and legacy Cargo slashes."""
    if not isinstance(expression, str) or not expression.strip():
        return False
    tokens = re.findall(r"\(|\)|[^\s()]+", expression.replace("/", " OR "))
    position = 0

    def atom() -> bool:
        nonlocal position
        if position >= len(tokens):
            raise ValueError("missing license")
        token = tokens[position]
        position += 1
        if token == "(":
            result = disjunction()
            if position >= len(tokens) or tokens[position] != ")":
                raise ValueError("unclosed license group")
            position += 1
            return result
        if token in {"AND", "OR", "WITH", ")"}:
            raise ValueError("unexpected operator")
        if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9.+-]*", token):
            raise ValueError("invalid license identifier")
        if position < len(tokens) and tokens[position] == "WITH":
            position += 1
            if position >= len(tokens) or tokens[position] in {"AND", "OR", "WITH", "(", ")"}:
                raise ValueError("missing exception")
            if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9.+-]*", tokens[position]):
                raise ValueError("invalid exception identifier")
            token += " WITH " + tokens[position]
            position += 1
        return token in ALLOWED or token in exceptions

    def conjunction() -> bool:
        nonlocal position
        result = atom()
        while position < len(tokens) and tokens[position] == "AND":
            position += 1
            following = atom()
            result = result and following
        return result

    def disjunction() -> bool:
        nonlocal position
        result = conjunction()
        while position < len(tokens) and tokens[position] == "OR":
            position += 1
            following = conjunction()
            result = result or following
        return result

    try:
        result = disjunction()
        return result and position == len(tokens)
    except ValueError:
        return False


def exceptions_for(ecosystem: str, name: str, version: str, notice: str) -> set[str]:
    """Only recognize package/version exceptions already ruled on in the ledger."""
    if ecosystem == "cargo" and (name, version) in MPL_RULINGS:
        row = rf"\|\s*`{re.escape(name)}`\s*\|\s*{re.escape(version)}\s*\|"
        if re.search(row, notice) and "unmodified" in notice and "Ruled acceptable" in notice:
            return {"MPL-2.0"}
    if ecosystem == "npm" and (name, version) in NPM_RULINGS:
        license_name = NPM_RULINGS[name, version]
        marker = f"`{name}` {version}"
        start = notice.find(marker)
        paragraph = notice[start:].split("\n\n", 1)[0] if start >= 0 else ""
        if license_name in paragraph and "Acceptable" in paragraph:
            return {license_name}
    return set()


def notice_files(root: Path, explicit: str | None = None) -> list[Path]:
    """Collect legal documents, avoiding arbitrary nested implementation files."""
    root = root.resolve()
    found: set[Path] = set()
    for path in root.iterdir():
        if path.is_symlink():
            continue
        if path.is_file() and path.stat().st_size and NOTICE_NAME.search(path.name):
            # LICENSE-APACHE-2.0 and COPYRIGHT-RUST.txt are legitimate names;
            # code files such as copying.rs are not standalone legal documents.
            if path.suffix.lower() not in {".rs", ".js", ".mjs", ".cjs", ".map", ".ts"}:
                found.add(path)
        elif path.is_dir() and path.name.lower() in {"license", "licenses", "licences", "legal"}:
            for child in path.rglob("*"):
                if child.is_file() and child.stat().st_size and not child.is_symlink() and child.resolve().is_relative_to(root):
                    if child.suffix.lower() in LEGAL_SUFFIXES:
                        found.add(child)
    if explicit:
        path = (root / explicit).resolve()
        if not path.is_relative_to(root) or not path.is_file() or not path.stat().st_size:
            raise ValueError(f"missing or external license_file: {root} / {explicit}")
        found.add(path)
    return sorted(found)


def command_json(command: list[str], root: Path) -> dict:
    """Run a read-only package metadata command, preserving actionable errors."""
    executable = shutil.which(command[0])
    if executable is None:
        raise ValueError(f"required executable not found: {command[0]}")
    result = subprocess.run([executable, *command[1:]], cwd=root, text=True, encoding="utf-8",
                            capture_output=True, check=False)
    if result.returncode:
        raise ValueError(f"{' '.join(command)} failed:\n{result.stderr.strip()}")
    return json.loads(result.stdout)


def cargo_dependencies(metadata: dict) -> list[Dependency]:
    """Audit the complete resolved graph, including build/dev/other targets."""
    workspace = set(metadata["workspace_members"])
    dependencies = []
    for package in metadata["packages"]:
        if package["id"] in workspace:
            if package.get("license") != "MIT OR Apache-2.0":
                raise ValueError(f"workspace license differs: {package['name']}")
            continue
        name, version = package["name"], package["version"]
        dependencies.append(Dependency(
            "cargo", name, version, package.get("license"),
            Path(package["manifest_path"]).parent, package.get("repository") or "",
            f"https://crates.io/api/v1/crates/{name}/{version}/download"
            if str(package.get("source", "")).startswith("registry+") else package.get("source") or "",
            package.get("license_file"),
        ))
    if not dependencies:
        raise ValueError("Cargo dependency inventory is empty")
    return dependencies


def npm_dependencies(inventory: dict) -> list[Dependency]:
    """Use installed manifests to pair aggregated pnpm names/versions/paths."""
    found: dict[tuple[str, str], Dependency] = {}
    for packages in inventory.values():
        for package in packages:
            versions = set(package["versions"])
            discovered = set()
            for directory in package["paths"]:
                root = Path(directory)
                manifest = json.loads((root / "package.json").read_text(encoding="utf-8"))
                name, version = manifest["name"], manifest["version"]
                if name != package["name"] or version not in versions:
                    raise ValueError(f"pnpm manifest/inventory mismatch: {root}")
                discovered.add(version)
                repository = manifest.get("repository") or package.get("homepage") or ""
                if isinstance(repository, dict):
                    repository = repository.get("url", "")
                license_name = manifest.get("license")
                if not isinstance(license_name, str):
                    # Inferred license labels may hide missing manifest metadata.
                    license_name = None
                found[name, version] = Dependency(
                    "npm", name, version, license_name, root, repository,
                    f"https://registry.npmjs.org/{name}/-/{name.split('/')[-1]}-{version}.tgz",
                )
            if discovered != versions:
                raise ValueError(f"pnpm inventory has uninstalled versions: {package['name']}")
    if not found:
        raise ValueError("npm dependency inventory is empty")
    return list(found.values())


def supplemental_notices(root: Path, item: Dependency) -> list[tuple[Path, str]]:
    """Read exact-version upstream documents omitted from published packages."""
    catalog_root = root / "scripts/release_licenses"
    catalog_path = catalog_root / f"{item.ecosystem}.json"
    if not catalog_path.exists():
        return []
    catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    documents = []
    for entry in catalog.get(f"{item.name}@{item.version}", []):
        path = (catalog_root / entry["path"]).resolve()
        if not path.is_relative_to(catalog_root.resolve()) or not path.is_file() or not path.stat().st_size:
            raise ValueError(f"missing or external catalog notice: {entry['path']}")
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != entry["sha256"] or not entry.get("source"):
            raise ValueError(f"notice provenance/hash mismatch: {entry['path']}")
        documents.append((path, entry["source"]))
    return documents


def dependency_notices(root: Path, item: Dependency) -> list[tuple[Path, str]]:
    """Combine package legal documents with verified upstream supplements."""
    local = [(path, f"{item.source}#/{path.relative_to(item.root.resolve()).as_posix()}")
             for path in notice_files(item.root, item.license_file)]
    return local + supplemental_notices(root, item)


def write_bundle(root: Path, output: Path, dependencies: list[Dependency]) -> list[dict]:
    """Write a self-contained attribution directory into an empty destination."""
    if output.exists() and any(output.iterdir()):
        raise ValueError(f"output directory must be empty: {output}")
    collected = [(item, dependency_notices(root, item)) for item in dependencies]
    for item, documents in collected:
        if not documents:
            raise ValueError(f"missing upstream notice: {item.ecosystem} {item.name}@{item.version}")
    output.mkdir(parents=True, exist_ok=True)
    for name in PROJECT_NOTICES:
        destination = output / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(root / name, destination)
    packages = []
    for item, documents in sorted(collected, key=lambda value: (value[0].ecosystem, value[0].name, value[0].version)):
        safe_name = re.sub(r"[^A-Za-z0-9._-]", "_", item.name)
        base = Path("dependencies") / item.ecosystem / f"{safe_name}-{item.version}"
        notices = []
        for index, (source, origin) in enumerate(documents):
            relative = base / f"{index + 1:02d}-{source.name}"
            destination = output / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            content = source.read_bytes()
            destination.write_bytes(content)
            notices.append({"path": relative.as_posix(), "sha256": hashlib.sha256(content).hexdigest(),
                            "source": origin})
        packages.append({"ecosystem": item.ecosystem, "name": item.name, "version": item.version,
                         "license": item.license, "repository": item.repository,
                         "source": item.source, "notices": notices})
    manifest = {"schema_version": 1,
                "scope": "Complete Cargo resolved graph (including build/dev/other targets) and installed pnpm production graph.",
                "packages": packages}
    (output / "DEPENDENCIES.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    (output / "README.txt").write_text(
        "FontCarver release licenses and third-party notices\n\n"
        "FontCarver is offered under MIT OR Apache-2.0; see the included texts.\n"
        "THIRD_PARTY.md records attributions and exact-version license rulings.\n"
        "DEPENDENCIES.json identifies dependency versions, license expressions,\n"
        "upstream source archives, and the SHA-256 of each retained notice.\n"
        "The inventory includes production npm dependencies and conservatively\n"
        "includes build/test and other-platform Rust\n"
        "packages; an inventory entry does not imply its code ships on this platform.\n"
        "Native libraries and bundler runtimes added to installers/AppImages are\n"
        "outside this source-package inventory and require artifact-level review.\n"
        "Linux host-library use and replacement are described in\n"
        "scripts/release_licenses/native/HOST-LIBRARIES.txt, with LGPL texts.\n"
        "Those notices describe separately installed libraries, not bundled code.\n"
        "For unmodified MPL dependencies, obtain corresponding source from each\n"
        "entry's exact-version source archive URL. Permissive alternatives are\n"
        "selected when offered alongside GPL/LGPL; see THIRD_PARTY.md.\n", encoding="utf-8",
    )
    return packages


def main() -> int:
    """Audit policy and notice presence before generating any release output."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, help="write release notices into an empty directory")
    args = parser.parse_args()
    try:
        for name in PROJECT_NOTICES:
            if not (ROOT / name).is_file() or not (ROOT / name).stat().st_size:
                raise ValueError(f"missing project notice: {name}")
        for name, expected in HOST_LICENSE_HASHES.items():
            if hashlib.sha256((ROOT / name).read_bytes()).hexdigest() != expected:
                raise ValueError(f"host-library license text differs from upstream: {name}")
        app = json.loads((ROOT / "app/package.json").read_text(encoding="utf-8"))
        if app.get("license") != "MIT OR Apache-2.0":
            raise ValueError("app/package.json must declare MIT OR Apache-2.0")
        dependencies = cargo_dependencies(command_json(
            ["cargo", "metadata", "--locked", "--offline", "--format-version", "1"], ROOT))
        npm_all = npm_dependencies(command_json(
            ["pnpm", "--dir", "app", "licenses", "list", "--json"], ROOT))
        npm_production = npm_dependencies(command_json(
            ["pnpm", "--dir", "app", "licenses", "list", "--prod", "--json"], ROOT))
        production_keys = {(item.name, item.version) for item in npm_production}
        if not production_keys.issubset({(item.name, item.version) for item in npm_all}):
            raise ValueError("npm production inventory is absent from full policy inventory")
        metadata_inventory = dependencies + npm_all
        dependencies += npm_production
        ledger = (ROOT / "THIRD_PARTY.md").read_text(encoding="utf-8")
        failures = []
        for item in metadata_inventory:
            identifier = f"{item.ecosystem} {item.name}@{item.version}"
            exceptions = exceptions_for(item.ecosystem, item.name, item.version, ledger)
            if not allowed_expression(item.license, exceptions):
                failures.append(f"{identifier}: unapproved or missing license {item.license!r}")
        for item in dependencies:
            if not dependency_notices(ROOT, item):
                failures.append(f"{item.ecosystem} {item.name}@{item.version}: missing upstream notice text")
        if failures:
            print("FAIL: release license audit\n" + "\n".join(f"- {failure}" for failure in failures), file=sys.stderr)
            return 1
        if args.output_dir:
            write_bundle(ROOT, args.output_dir.resolve(), dependencies)
        cargo_count = sum(item.ecosystem == "cargo" for item in dependencies)
        print(f"PASS: source-package policy verified for {cargo_count} Cargo and {len(npm_all)} npm versions; "
              f"notice documents checked for {cargo_count} Cargo and {len(npm_production)} production npm versions")
        if args.output_dir:
            print(f"Notice bundle: {args.output_dir}")
        return 0
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
