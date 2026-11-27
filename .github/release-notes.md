### Get it

<!-- The release workflow replaces {version}, {tag} and {repo}. -->

| System | Download |
| --- | --- |
| **Windows** 10 and 11, x64 | [`fastemu-{version}-windows-x64-setup.exe`](https://github.com/{repo}/releases/download/{tag}/fastemu-{version}-windows-x64-setup.exe): installs FastEmu for your account, with no administrator rights. Or [`fastemu-{version}-windows-x64-portable.zip`](https://github.com/{repo}/releases/download/{tag}/fastemu-{version}-windows-x64-portable.zip): unpack it anywhere and run `fastemu.exe` |
| **Linux**, x64 | [`fastemu-{version}-linux-x64.tar.gz`](https://github.com/{repo}/releases/download/{tag}/fastemu-{version}-linux-x64.tar.gz), or [`fastemu-{version}-linux-x64.deb`](https://github.com/{repo}/releases/download/{tag}/fastemu-{version}-linux-x64.deb) for Debian and Ubuntu |
| **Linux**, Arm64 | [`fastemu-{version}-linux-arm64.tar.gz`](https://github.com/{repo}/releases/download/{tag}/fastemu-{version}-linux-arm64.tar.gz) |
| **macOS** 11 or later, Apple silicon and Intel | [`fastemu-{version}-macos-universal.zip`](https://github.com/{repo}/releases/download/{tag}/fastemu-{version}-macos-universal.zip): unpack it and move FastEmu to Applications |
| **Android** 12 or later | [`fastemu-{version}-android.apk`](https://github.com/{repo}/releases/download/{tag}/fastemu-{version}-android.apk): allow installing from this source when Android asks |
| **Any Linux**, sandboxed, x64 | [`fastemu-{version}-linux-x64.flatpak`](https://github.com/{repo}/releases/download/{tag}/fastemu-{version}-linux-x64.flatpak): `flatpak install ./fastemu-{version}-linux-x64.flatpak`, which fetches the freedesktop runtime from Flathub the first time |
| **Any Linux**, sandboxed, Arm64 | [`fastemu-{version}-linux-arm64.flatpak`](https://github.com/{repo}/releases/download/{tag}/fastemu-{version}-linux-arm64.flatpak): `flatpak install ./fastemu-{version}-linux-arm64.flatpak`, which fetches the freedesktop runtime from Flathub the first time |

**First launch on macOS.** The app is not notarized: open it once from System Settings > Privacy & Security > Open Anyway, or run `xattr -dr com.apple.quarantine /Applications/FastEmu.app`.

**First launch on Windows.** The setup and the program are not signed: if SmartScreen warns, choose More info > Run anyway.

**Games.** FastEmu does not include any. Add `.jar` files, with their `.jad` files when you have them, with Add game in the library. The [README](https://github.com/{repo}/blob/{tag}/README.md#usage) tells how.

Verify downloads with `SHA256SUMS`: `sha256sum -c SHA256SUMS --ignore-missing` on Linux and macOS, or `Get-FileHash` on Windows, compared with the line of the file.
