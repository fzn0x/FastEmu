### Get it

The release workflow replaces `{version}`, `{tag}` and `{repo}`.

| System | Download |
| --- | --- |
| **Windows** 10 and 11, x64 | [`fastemu-{version}-windows-x64.zip`](https://github.com/{repo}/releases/download/{tag}/fastemu-{version}-windows-x64.zip): unpack it and run `fastemu.exe` |
| **Linux**, x64 | [`fastemu-{version}-linux-x64.tar.gz`](https://github.com/{repo}/releases/download/{tag}/fastemu-{version}-linux-x64.tar.gz) |
| **Linux**, Arm64 | [`fastemu-{version}-linux-arm64.tar.gz`](https://github.com/{repo}/releases/download/{tag}/fastemu-{version}-linux-arm64.tar.gz) |
| **macOS**, Apple silicon | [`fastemu-{version}-macos-arm64.tar.gz`](https://github.com/{repo}/releases/download/{tag}/fastemu-{version}-macos-arm64.tar.gz) |
| **macOS**, Intel | [`fastemu-{version}-macos-x64.tar.gz`](https://github.com/{repo}/releases/download/{tag}/fastemu-{version}-macos-x64.tar.gz) |
| **Android** | [`fastemu-{version}-android.apk`](https://github.com/{repo}/releases/download/{tag}/fastemu-{version}-android.apk): allow installing from this source when Android asks |

**First launch on macOS.** The app is not notarized: open it once from System Settings > Privacy & Security > Open Anyway, or run `xattr -dr com.apple.quarantine fastemu`.

**First launch on Windows.** The program is not signed: if SmartScreen warns, choose More info > Run anyway.

**Games.** FastEmu does not include any. Add `.jar` files, with their `.jad` files when you have them, from the game list. The [README](https://github.com/{repo}/blob/{tag}/README.md#usage) tells how.

Verify downloads with `SHA256SUMS`: `sha256sum -c SHA256SUMS --ignore-missing` on Linux and macOS, or `Get-FileHash` on Windows, compared with the line of the file.
