<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="assets/logo-light.svg">
    <img src="assets/logo.svg" width="480" alt="FastEmu">
  </picture>
</p>

<p align="center">
  Emulator Java ME yang ditulis dengan Rust.<br>
  Menjalankan banyak game MIDP 2.0 sekaligus dalam satu jendela.
</p>

<p align="center">
  <a href="https://github.com/fzn0x/FastEmu/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/fzn0x/FastEmu/actions/workflows/ci.yml/badge.svg"></a>
  <a href="#lisensi"><img alt="GPL-3.0-or-later" src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg"></a>
  <a href="https://fastemu.functionx.dev"><img alt="Situs" src="https://img.shields.io/badge/site-fastemu.functionx.dev-blue.svg"></a>
</p>

---

FastEmu menjalankan game Java ME, yaitu file `.jar` dan `.jad` buatan
untuk ponsel tahun 2000-an, di Windows, Linux, macOS, dan Android. FastEmu
mengimplementasikan CLDC 1.1 dan MIDP 2.0 dengan mesin virtual sendiri,
jadi Java tidak perlu terpasang. Setiap game berjalan di mesin virtualnya
sendiri dengan memori, thread, data simpanan, dan suara sendiri, dan satu
jendela menampilkan sebanyak apa pun game yang Anda jalankan. Game yang
crash atau macet tidak menghentikan game di sebelahnya.

[Pasang](#pasang) · [Penggunaan](#penggunaan) · [API yang didukung](#api-yang-didukung) ·
[Build dari sumber](#build-dari-sumber) · [Kontribusi](CONTRIBUTING.md) ·
[Changelog](CHANGELOG.md) · [English](README.md)

## Pasang

Unduh file untuk sistem Anda dari
[halaman Releases](https://github.com/fzn0x/FastEmu/releases/latest). Setiap
rilis punya file `SHA256SUMS` untuk memeriksa unduhan.

| Sistem | Caranya |
| --- | --- |
| Windows | Ekstrak `fastemu-<versi>-windows-x64.zip` di mana saja dan jalankan `fastemu.exe`. Jika SmartScreen memperingatkan, pilih More info, lalu Run anyway: programnya belum ditandatangani. |
| Linux | Ekstrak `fastemu-<versi>-linux-x64.tar.gz` (atau `-linux-arm64`) di mana saja dan jalankan `fastemu`, atau pasang `fastemu-<versi>-linux-x64.deb` di Debian dan Ubuntu. Butuh driver Vulkan dan sesi Wayland atau X11. |
| macOS | Ekstrak `fastemu-<versi>-macos-universal.zip` dan pindahkan aplikasinya ke Applications. Aplikasinya belum dinotarisasi: buka sekali lewat System Settings, Privacy & Security, Open Anyway. |
| Android | Buka `fastemu-<versi>-android.apk` dari berkas Anda. Android akan meminta izin memasang dari sumber itu pada pertama kali. |

FastEmu tidak menyertakan game. Pakai file yang Anda miliki atau yang
dibagikan gratis oleh pembuatnya.

## Penggunaan

**Menambah game.** Seret file `.jar` atau `.jad` ke jendela, pilih **+**
di atas perpustakaan, atau berikan file lewat baris perintah, yang
menambahkan lalu menjalankan setiap file:

```sh
fastemu snake.jar tetris.jad
```

Di Android, pilih **+** dan pilih file dari penyimpanan, atau buka file
dari pengelola file dengan FastEmu. FastEmu membaca nama, vendor, versi,
dan ikon dari manifes atau deskriptor, lalu menampilkan game di
perpustakaannya. Klik sebuah game untuk menjalankannya.

**Menjalankan beberapa game.** Jalankan game lain dari perpustakaan saat
satu game sedang berjalan, dan game itu terbuka di sebelahnya. Game yang
sama bisa dijalankan dua kali; setiap salinan menyimpan datanya sendiri.
Semua game memutar suaranya, dan masing-masing bisa dibisukan. Jendela
menampilkan game yang berjalan dalam kisi berukuran sama, atau game yang
disorot dalam ukuran besar dengan yang lain di jalur sampingnya.

**Tombol.** Keyboard memainkan game yang disorot:

| Tombol ponsel | Keyboard |
| --- | --- |
| 0 sampai 9 | `0` sampai `9` di baris angka dan numpad |
| `*` dan `#` | `*` dan `#` (`Shift+8`, `Shift+3`), atau `*` dan `/` di numpad |
| Atas, bawah, kiri, kanan | Tombol panah |
| Fire | Enter atau Spasi |
| Soft key kiri dan kanan | F1 dan F2 |
| Clear | Backspace |

Semua pemetaan bisa diubah di pengaturan. Setiap game juga punya keypad
ponsel di bawah layarnya, yang ditahan dengan mouse atau jari, dan game
dengan profil ponsel layar sentuh menerima klik, ketukan, dan geseran di
layarnya. Saat game menampilkan kotak teks, ketikan masuk sebagai teks; di
Android terbuka dialog dengan keyboard sistem. Kode tombol yang dilihat
game berasal dari profil ponselnya, karena pembuat ponsel memakai kode
berbeda: tata letak Nokia, Siemens, dan Motorola sudah tersedia.

**Tanpa jendela.** `fastemu run` menjalankan game untuk pengujian dan
skrip, menekan tombol dan menyentuh layar dari skrip, dan menyimpan
gambar:

```sh
fastemu run snake.jar --frames 300 --keys "wait:1500 5" --screenshot snake.png
```

[Dokumentasi](https://fastemu.functionx.dev) memuat daftar tombol dan
pintasan lengkap, pengaturan, profil ponsel, lokasi data simpanan, dan
semua opsi `fastemu run`.

## API yang didukung

FastEmu menargetkan platform yang dipakai game pada masanya:

- CLDC 1.1 dan subset `java.lang`, `java.util`, dan `java.io`-nya
- MIDP 2.0: layar `lcdui` dan `Canvas`, record store, Generic Connection
  Framework, dan siklus hidup MIDlet
- Game API MIDP 2.0
- Mobile Media API (MMAPI, JSR 135) untuk suara dan MIDI
- Nokia UI API

Game mana yang berjalan dicatat di
[daftar kompatibilitas](https://fastemu.functionx.dev), yang disusun dari
laporan pengguna. Game yang tidak berjalan sebaiknya dilaporkan lewat
[laporan game](https://github.com/fzn0x/FastEmu/issues/new?template=game_report.yml).

## Build dari sumber

Anda butuh [rustup](https://rustup.rs). Repositori ini mengunci toolchain-nya
di `rust-toolchain.toml`, jadi Rust yang tepat terpasang pada build pertama.

```sh
git clone https://github.com/fzn0x/FastEmu.git
cd FastEmu
cargo test            # crate emulator
cargo run -p fastemu  # aplikasi desktop
```

Di Linux, aplikasi desktop juga butuh compiler C dan paket pengembangan
GPUI: fontconfig, xkbcommon, Wayland, X11, dan Vulkan. Di macOS butuh
Xcode, yang toolchain Metal-nya mengompilasi shader GPUI. Tes mesin virtual
membandingkan hasilnya dengan JDK asli, jadi pasang JDK 8 atau lebih baru
untuk itu. [CONTRIBUTING.md](CONTRIBUTING.md) memuat daftar paket yang
tepat, dan [docs/building-android.md](docs/building-android.md) memuat
langkah aplikasi Android.

## Cara FastEmu dibuat

Workspace Rust berisi crate-crate kecil. `fastemu-core` memuat tipe yang
dipakai bersama oleh runtime dan aplikasi: frame, event input, profil
perangkat, dan peluncur yang menjalankan game. `fastemu-jvm` adalah mesin
virtual beserta pustaka kelasnya, `fastemu-gfx` grafis 2D, `fastemu-audio`
suara dan MIDI, dan `fastemu-midp` API MIDP, game, media, dan Nokia yang
dipanggil game. `fastemu-ui` adalah antarmuka GPUI yang dipakai bersama
oleh program desktop `fastemu` dan aplikasi Android `fastemu-android`.

## Penghargaan

[MicroEmulator](https://github.com/barteo/microemu) dan
[J2ME Loader](https://github.com/nikita36078/J2ME-Loader) menjaga game-game
ini tetap bisa dimainkan selama lebih dari sepuluh tahun. FastEmu mengikuti
perilaku keduanya di tempat game bergantung padanya, dan berutang pada
pengetahuan mereka tentang kejanggalan mana yang penting.

Antarmukanya dibangun dengan [GPUI](https://github.com/zed-industries/zed),
kerangka UI editor Zed. GPUI di Android berjalan di atas port Zed untuk
Android oleh Dylan Murzello.

Crate Rust dan font yang dipakai FastEmu, beserta lisensinya, tercantum di
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). Orang-orang yang membantu
ada di [CONTRIBUTORS.md](CONTRIBUTORS.md).

## Lisensi

FastEmu adalah perangkat lunak bebas di bawah [GNU General Public License
v3.0 atau yang lebih baru](LICENSE).
