#!/usr/bin/env python3
"""Pre-push checks. Run before every push. Exits non-zero on first failure."""

import subprocess
import sys


def run(cmd: list[str], label: str) -> bool:
    """Run command, print result, return True if pass."""
    print(f"  {label}...", end=" ", flush=True)
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode == 0:
        print("ok")
        return True
    print("FAIL")
    if result.stdout:
        print(result.stdout)
    if result.stderr:
        print(result.stderr)
    return False


def main() -> int:
    checks = [
        (["cargo", "fmt", "--check"], "fmt"),
        (["cargo", "clippy", "--", "-D", "warnings"], "clippy"),
        (["cargo", "test"], "test"),
    ]

    print("prepush checks:")
    for cmd, label in checks:
        if not run(cmd, label):
            print(f"\nfailed: {label}")
            return 1

    print("\nall checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
