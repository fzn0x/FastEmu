#!/usr/bin/env python3
"""Writes FastEmu's AppStream metadata with its releases, for the Flatpak:

    python3 packaging/flatpak/metainfo.py OUT

It reads packaging/flatpak/io.github.fzn0x.fastemu.metainfo.xml and puts
a <release> in place of its <releases/> for each "## [X.Y.Z] - YYYY-MM-DD"
heading of CHANGELOG.md, newest first, as the changelog lists them. Run
it from the repository's root. Only the standard library is used.
"""

import re
import sys

TEMPLATE = "packaging/flatpak/io.github.fzn0x.fastemu.metainfo.xml"
HEADING = re.compile(r"^## \[(\d+\.\d+\.\d+)\] - (\d{4}-\d{2}-\d{2})$", re.MULTILINE)


def main(args):
    if len(args) != 1:
        print(__doc__.strip().split("\n\n")[1], file=sys.stderr)
        return 2
    with open("CHANGELOG.md", encoding="utf-8") as f:
        releases = HEADING.findall(f.read())
    if not releases:
        print("metainfo: CHANGELOG.md has no released version", file=sys.stderr)
        return 1
    with open(TEMPLATE, encoding="utf-8") as f:
        template = f.read()
    if template.count("<releases/>") != 1:
        print(f"metainfo: {TEMPLATE} must hold one <releases/>", file=sys.stderr)
        return 1
    lines = ["<releases>"]
    for version, date in releases:
        lines.append(f'    <release version="{version}" date="{date}">')
        lines.append(f'      <url type="details">https://github.com/fzn0x/FastEmu/releases/tag/v{version}</url>')
        lines.append("    </release>")
    lines.append("  </releases>")
    with open(args[0], "w", encoding="utf-8", newline="\n") as f:
        f.write(template.replace("<releases/>", "\n".join(lines)))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
