#!/usr/bin/env python3
# MODULE_CONTRACT
# MODULE_ID: M-CI-RUNTIME-GUARD
# PURPOSE: Production runtime panic guard — blocks unsafe unwrap/expect/panic markers outside test code
# SCOPE: Rust production source scan, cfg(test) skipping, inline allow marker, self-test fixtures
# DEPENDS: N/A
# LINKS: scripts/ci.sh, docs/phases/Phase-3.xml

# START_MODULE_MAP
# main — CLI entrypoint for repository scan and self-test
# collect_violations — Finds disallowed panic markers in production Rust code
# is_test_attribute — Detects Rust test-only attributes
# brace_delta — Tracks Rust block nesting for test block skipping
# run_self_test — Verifies positive and negative guard fixtures
# END_MODULE_MAP

# START_CHANGE_SUMMARY
# LAST_CHANGE: [v1.1.0 - Named runtime guard remediation hint]
# END_CHANGE_SUMMARY

from __future__ import annotations

import argparse
import re
import sys
import tempfile
import textwrap
from dataclasses import dataclass
from pathlib import Path


DENIED_MARKERS: tuple[tuple[str, re.Pattern[str]], ...] = (
    (".unwrap()", re.compile(r"\.unwrap\s*\(")),
    (".unwrap_err()", re.compile(r"\.unwrap_err\s*\(")),
    (".expect()", re.compile(r"\.expect\s*\(")),
    ("panic!", re.compile(r"\bpanic!\s*\(")),
    ("todo!", re.compile(r"\btodo!\s*\(")),
    ("unimplemented!", re.compile(r"\bunimplemented!\s*\(")),
)
ALLOW_MARKER = "runtime-guard: allow"
RUNTIME_GUARD_FIX_HINT_PREFIX = "Add error handling or use '"
RUNTIME_GUARD_FIX_HINT_SUFFIX = "' with a justification for intentional production panics."
RUNTIME_GUARD_FIX_HINT = (
    RUNTIME_GUARD_FIX_HINT_PREFIX + ALLOW_MARKER + RUNTIME_GUARD_FIX_HINT_SUFFIX
)


@dataclass(frozen=True)
class Violation:
    path: Path
    line_no: int
    marker: str
    line: str


# START_public_api


# START_CONTRACT_main
# PURPOSE: Run runtime panic guard scan or self-test from command line
# INPUTS: { argv: list[str] | None — CLI args }
# OUTPUTS: { int — process exit code }
# SIDE_EFFECTS: prints scan results to stdout/stderr
# START_main
def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Block production Rust panic markers.")
    parser.add_argument("--root", default=".", help="Repository root to scan")
    parser.add_argument("--self-test", action="store_true", help="Run guard fixture tests")
    args = parser.parse_args(argv)

    if args.self_test:
        run_self_test()
        print("[CI][runtime_guard][SELF_TEST] fixtures passed")
        return 0

    root = Path(args.root).resolve()
    violations = collect_violations(root)
    if violations:
        print("[CI][runtime_guard][SCAN] Production panic markers found:", file=sys.stderr)
        for violation in violations:
            rel = violation.path.relative_to(root)
            print(
                f"{rel}:{violation.line_no}: {violation.marker}: {violation.line}",
                file=sys.stderr,
            )
        print(RUNTIME_GUARD_FIX_HINT, file=sys.stderr)
        return 1

    print("[CI][runtime_guard][SCAN] No production panic markers found")
    return 0


# END_main


# START_CONTRACT_collect_violations
# PURPOSE: Find disallowed panic markers in production Rust files while skipping test-only blocks
# INPUTS: { root: Path — repository root }
# OUTPUTS: { list[Violation] — sorted guard violations }
# START_collect_violations
def collect_violations(root: Path) -> list[Violation]:
    violations: list[Violation] = []
    for path in production_rust_files(root):
        violations.extend(scan_rust_file(root, path))
    return violations


# END_collect_violations


# END_public_api


# START_CONTRACT_production_rust_files
# PURPOSE: Enumerate Rust files governed by the production runtime panic guard
# INPUTS: { root: Path — repository root }
# OUTPUTS: { list[Path] — sorted Rust source files }
# START_production_rust_files
def production_rust_files(root: Path) -> list[Path]:
    files = sorted(root.joinpath("src").rglob("*.rs")) if root.joinpath("src").exists() else []
    build_rs = root.joinpath("build.rs")
    if build_rs.exists():
        files.append(build_rs)
    return [path for path in files if "target" not in path.parts]


# END_production_rust_files


# START_CONTRACT_scan_rust_file
# PURPOSE: Scan one Rust file for disallowed markers outside test-only regions
# INPUTS: { root: Path }, { path: Path — Rust source file }
# OUTPUTS: { list[Violation] }
# START_scan_rust_file
def scan_rust_file(root: Path, path: Path) -> list[Violation]:
    violations: list[Violation] = []
    pending_test_attr = False
    skip_until_depth: int | None = None
    depth = 0

    for line_no, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        stripped = raw_line.strip()

        if is_test_attribute(stripped):
            pending_test_attr = True
            continue

        delta = brace_delta(raw_line)
        if skip_until_depth is not None:
            depth += delta
            if depth <= skip_until_depth:
                skip_until_depth = None
            continue

        if pending_test_attr:
            if "{" in raw_line:
                skip_until_depth = depth
                depth += delta
                if depth <= skip_until_depth:
                    skip_until_depth = None
                pending_test_attr = False
                continue
            if stripped and not stripped.startswith("#["):
                pending_test_attr = False

        if ALLOW_MARKER not in raw_line:
            for marker, pattern in DENIED_MARKERS:
                if pattern.search(raw_line):
                    violations.append(
                        Violation(
                            path=path,
                            line_no=line_no,
                            marker=marker,
                            line=raw_line.strip(),
                        )
                    )

        depth += delta

    return violations


# END_scan_rust_file


# START_CONTRACT_is_test_attribute
# PURPOSE: Detect Rust attributes that mark the following item as test-only
# INPUTS: { stripped_line: str — trimmed Rust source line }
# OUTPUTS: { bool }
# START_is_test_attribute
def is_test_attribute(stripped_line: str) -> bool:
    return (
        stripped_line.startswith("#[cfg(test)]")
        or stripped_line.startswith("#[test]")
        or stripped_line.startswith("#[tokio::test")
        or stripped_line.startswith("#[async_std::test")
    )


# END_is_test_attribute


# START_CONTRACT_brace_delta
# PURPOSE: Estimate Rust brace depth changes after stripping strings and line comments
# INPUTS: { line: str — Rust source line }
# OUTPUTS: { int — opening braces minus closing braces }
# START_brace_delta
def brace_delta(line: str) -> int:
    code = strip_strings_and_comments(line)
    return code.count("{") - code.count("}")


# END_brace_delta


# START_CONTRACT_strip_strings_and_comments
# PURPOSE: Remove quoted strings, char literals, and line comments before brace counting
# INPUTS: { line: str — Rust source line }
# OUTPUTS: { str — source-like line with non-code text removed }
# START_strip_strings_and_comments
def strip_strings_and_comments(line: str) -> str:
    out: list[str] = []
    index = 0
    in_string = False
    in_char = False
    escaped = False
    while index < len(line):
        char = line[index]
        next_char = line[index + 1] if index + 1 < len(line) else ""
        if not in_string and not in_char and char == "/" and next_char == "/":
            break
        if in_string:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            index += 1
            continue
        if in_char:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == "'":
                in_char = False
            index += 1
            continue
        if char == '"':
            in_string = True
            index += 1
            continue
        if char == "'":
            in_char = True
            index += 1
            continue
        out.append(char)
        index += 1
    return "".join(out)


# END_strip_strings_and_comments


# START_CONTRACT_run_self_test
# PURPOSE: Verify guard behavior for production violations, test skips, fallbacks, and inline allow markers
# OUTPUTS: { None — raises AssertionError on fixture failure }
# SIDE_EFFECTS: writes temporary Rust fixtures
# START_run_self_test
def run_self_test() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        src = root.joinpath("src")
        src.mkdir()

        write_fixture(
            src.joinpath("lib.rs"),
            """
            pub fn safe(value: Option<String>) -> String {
                value.unwrap_or_default()
            }

            #[cfg(test)]
            mod tests {
                #[test]
                fn fixture_can_unwrap() {
                    Some(1).unwrap();
                    panic!("allowed in tests");
                }
            }
            """,
        )
        assert collect_violations(root) == []

        write_fixture(src.joinpath("lib.rs"), "pub fn bad(v: Option<u8>) -> u8 { v.unwrap() }\n")
        violations = collect_violations(root)
        assert len(violations) == 1
        assert violations[0].marker == ".unwrap()"

        write_fixture(
            src.joinpath("lib.rs"),
            f"pub fn intentional() {{ panic!(\"stop\"); // {ALLOW_MARKER}: fatal invariant }}\n",
        )
        assert collect_violations(root) == []


# END_run_self_test


def write_fixture(path: Path, content: str) -> None:
    path.write_text(textwrap.dedent(content).lstrip(), encoding="utf-8")


if __name__ == "__main__":
    raise SystemExit(main())
