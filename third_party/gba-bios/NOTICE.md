# Bundled GBA BIOS

Lycan ships the open-source guest BIOS from
[ez-me/gba-bios release 1.0](https://github.com/ez-me/gba-bios/releases/tag/1.0),
revision `bde9138`. The upstream project attributes its implementation to
VisualBoyAdvance-M, Normmatt, and ReGBA. See the preserved upstream `README.md`.

The BIOS is licensed under **GNU GPL version 2**, separately from Lycan's own
MIT-licensed source. The license is reproduced in `LICENSE`. The unmodified
source snapshot, Makefile, and linker specifications accompany the BIOS in this
directory. Its upstream build requires devkitARM and libtonc as described by the
Makefile; the bundled release binary has not been rebuilt from those sources.

Binary identity:

- Size: 16,384 bytes
- SHA-1: `598c0b2c6c5d15bbba218773574c9e7856d141f3`
- SHA-256: `661a9afb93624f2c5e77d07dab137bd8d23cff79e8639c62fe621b49bb064749`

The BIOS is executed as guest firmware. It is installed by default in the
native executable and WebAssembly module. Choosing **Change BIOS** on the launch
screen or **Replace BIOS…** in the Game menu replaces it for the current app
session. User-selected BIOS files are not added to the distributed application.

Distribute this entire directory with native download packages. Browser builds
publish it under `third_party/gba-bios/`, including this notice, the license,
and source. This notice does not relicense the BIOS under the MIT License.
