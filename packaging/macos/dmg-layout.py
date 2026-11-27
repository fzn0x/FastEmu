#!/usr/bin/env python3
"""Writes the .DS_Store that lays out the Finder window of FastEmu's disk
image, for packaging/macos/package.sh:

    python3 packaging/macos/dmg-layout.py OUT APP
    python3 packaging/macos/dmg-layout.py --check

OUT is the .DS_Store to write and APP the bundle's file name, such as
FastEmu.app. The window shows the app on the left and the link to
/Applications on the right, with large icons and no toolbar or sidebar.
Finder reads the file when the image opens, so making it needs no
AppleScript, no Finder and no screen, and works on a CI runner.

The format is a buddy-allocated file holding one B-tree. This is a port
of MyGo's cmd/mygo/dsstore.go (MIT; see THIRD_PARTY_NOTICES.md), whose
output is byte for byte what the ds_store package of dmgbuild writes, a
layout Finder is known to honor. The window's settings are binary
property lists from plistlib, as ds_store writes them. --check builds the
layout of MyGo's test and compares it with the SHA-256 of the file
ds_store wrote for it.

Only the standard library is used.
"""

import hashlib
import plistlib
import struct
import sys

# The Finder window: its bottom-left corner from the bottom of the screen,
# its size, the icons' size and the centers of the two icons, in points
# from the window's top-left corner.
WINDOW_X, WINDOW_Y = 200, 400
WINDOW_WIDTH, WINDOW_HEIGHT = 660, 400
ICON_SIZE = 128
APP_X, APP_Y = 180, 190
APPLICATIONS_X, APPLICATIONS_Y = 480, 185

# The layout of MyGo's TestDSStoreMatchesDMGBuild, for "Hello MyGo.app",
# and the SHA-256 of testdata/dmg.DS_Store, which ds_store wrote for it.
CHECK_APP = "Hello MyGo.app"
CHECK_SHA256 = "f05b6c18c7e062fa830d315b30d8254d9587af005479c784205ff3a4f1451b30"


def record(name, code, kind, value):
    """One record: a property (code) of a file in the folder, or of the
    folder itself when name is ".". kind is b"long", b"blob" or b"type"."""
    units = name.encode("utf-16-be")
    out = struct.pack(">I", len(units) // 2) + units + code + kind
    if kind == b"long":
        return out + struct.pack(">I", value)
    if kind == b"blob":
        return out + struct.pack(">I", len(value)) + value
    return out + value


def plist(values):
    return plistlib.dumps(values, fmt=plistlib.FMT_BINARY, sort_keys=True)


def icon_location(name, x, y):
    """Where the center of a file's icon goes."""
    return (name, b"Iloc", b"blob", struct.pack(">II", x, y) + b"\xff" * 6 + b"\x00" * 2)


def window(app):
    """The records of a window holding app and an Applications link."""
    browser = {
        "ShowStatusBar": False,
        "WindowBounds": "{{%d, %d}, {%d, %d}}" % (WINDOW_X, WINDOW_Y, WINDOW_WIDTH, WINDOW_HEIGHT),
        "ContainerShowSidebar": False,
        "PreviewPaneVisibility": False,
        "SidebarWidth": 180,
        "ShowTabView": False,
        "ShowToolbar": False,
        "ShowPathbar": False,
        "ShowSidebar": False,
    }
    icon_view = {
        "viewOptionsVersion": 1,
        "backgroundType": 0,
        "backgroundColorRed": 1.0,
        "backgroundColorGreen": 1.0,
        "backgroundColorBlue": 1.0,
        "gridOffsetX": 0.0,
        "gridOffsetY": 0.0,
        "gridSpacing": 100.0,
        "arrangeBy": "none",
        "showIconPreview": False,
        "showItemInfo": False,
        "labelOnBottom": True,
        "textSize": 16.0,
        "iconSize": float(ICON_SIZE),
        "scrollPositionX": 0.0,
        "scrollPositionY": 0.0,
    }
    return [
        (".", b"vSrn", b"long", 1),
        (".", b"bwsp", b"blob", plist(browser)),
        (".", b"icvp", b"blob", plist(icon_view)),
        (".", b"icvl", b"type", b"icnv"),
        icon_location(app, APP_X, APP_Y),
        icon_location("Applications", APPLICATIONS_X, APPLICATIONS_Y),
    ]


def ds_store(records):
    """The .DS_Store file holding records.

    After a 4-byte prefix comes the allocator's address space: its header
    at 0, the B-tree's header ("DSDB") at 0x20, the allocator's root block
    at 0x800 and one B-tree leaf at 0x2000. The records must fit in that
    leaf, which is plenty for the few of a disk image's window."""
    records = sorted(records, key=lambda r: (r[0].lower(), r[1]))
    page_size = 0x1000
    node = struct.pack(">II", 0, len(records))  # a leaf: no child pointer
    node += b"".join(record(*r) for r in records)
    if len(node) > page_size:
        raise ValueError(f"the records need {len(node)} bytes, more than one page")

    dsdb, root, leaf = 0x20, 0x800, 0x2000
    mem = bytearray(0x4000)  # allocator addresses are relative to this

    def put(offset, *numbers):
        struct.pack_into(">" + "I" * len(numbers), mem, offset, *numbers)

    mem[0:4] = b"Bud1"
    put(4, root, 0x800, root)  # the root block, its size, the root again
    mem[16:32] = bytes([0, 0, 0x10, 0x0C, 0, 0, 0, 0x87, 0, 0, 0x20, 0x0B, 0, 0, 0, 0])

    # The B-tree: its root node is block 2, of one node, a page each.
    put(dsdb, 2, 0, len(records), 1, page_size)

    # The allocator's root: the blocks, each an offset with its log2 size,
    # in a list padded to 256 entries.
    put(root, 3)
    for i, address in enumerate((root | 11, dsdb | 5, leaf | 13)):
        put(root + 8 + 4 * i, address)
    p = root + 8 + 256 * 4
    put(p, 1)
    mem[p + 4] = 4
    mem[p + 5 : p + 9] = b"DSDB"
    put(p + 9, 1)  # the B-tree's header is block 1
    p += 13
    # Free lists by log2 size. Carving the blocks above out of the initial
    # 2 GiB leaves one free block of each other size, at an offset equal to
    # its size.
    for width in range(32):
        if width < 6 or width in (11, 13, 31):
            p += 4
            continue
        put(p, 1, 1 << width)
        p += 8

    mem[leaf : leaf + len(node)] = node
    return b"\x00\x00\x00\x01" + bytes(mem)


def main(args):
    if args == ["--check"]:
        digest = hashlib.sha256(ds_store(window(CHECK_APP))).hexdigest()
        if digest != CHECK_SHA256:
            print(f"dmg-layout: the layout of {CHECK_APP!r} has SHA-256 {digest}, not {CHECK_SHA256}", file=sys.stderr)
            return 1
        print("dmg-layout: the layout matches what ds_store writes")
        return 0
    if len(args) != 2:
        print(__doc__.strip().split("\n\n")[1], file=sys.stderr)
        return 2
    out, app = args
    with open(out, "wb") as f:
        f.write(ds_store(window(app)))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
