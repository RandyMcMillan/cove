#!/usr/bin/env python3
"""Fix known corruption in ios/Cove.xcodeproj/project.pbxproj."""
import re
import subprocess
import sys

PROJECT_PATH = "ios/Cove.xcodeproj/project.pbxproj"


def fix_build_rust_configs(content: str) -> str:
    """Repair incomplete Build Rust XCBuildConfiguration entries."""
    # The broken insertion is missing the 'name = ...;' line and closing '};'
    # for both the Debug and Release Build Rust configurations.
    debug_pattern = re.compile(
        r"(C0A000122EF0000000000002 /\* Debug \*/ = \{.*?"
        r"PRODUCT_NAME = \"Build Rust\";\s*\};)\s*"
        r"(C0A000132EF0000000000003 /\* Release \*/ = \{)",
        re.DOTALL,
    )

    def fix_debug(match: re.Match) -> str:
        return (
            match.group(1)
            + "\n\t\t\tname = Debug;\n\t\t};\n\n\t\t"
            + match.group(2)
        )

    content, debug_count = debug_pattern.subn(fix_debug, content, count=1)
    if debug_count:
        print("Fixed Build Rust Debug config")

    release_pattern = re.compile(
        r"(C0A000132EF0000000000003 /\* Release \*/ = \{.*?"
        r"PRODUCT_NAME = \"Build Rust\";\s*\};)\s*"
        r"(/\* End XCBuildConfiguration section \*/)",
        re.DOTALL,
    )

    def fix_release(match: re.Match) -> str:
        return (
            match.group(1)
            + "\n\t\t\tname = Release;\n\t\t};\n\n\t"
            + match.group(2)
        )

    content, release_count = release_pattern.subn(fix_release, content, count=1)
    if release_count:
        print("Fixed Build Rust Release config")

    return content


def validate(path: str) -> int:
    result = subprocess.run(
        ["python3", "scripts/validate-pbxproj.py", path],
        capture_output=True,
        text=True,
    )
    print(result.stdout.strip())
    if result.stderr:
        print(result.stderr.strip(), file=sys.stderr)
    return result.returncode


def main() -> int:
    with open(PROJECT_PATH, "r") as f:
        content = f.read()

    content = fix_build_rust_configs(content)

    with open(PROJECT_PATH, "w") as f:
        f.write(content)

    return validate(PROJECT_PATH)


if __name__ == "__main__":
    sys.exit(main())
