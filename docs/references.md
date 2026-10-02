# Reference implementations

Exact revisions inspected while implementing the emulator.

| Project | Revision | Used for |
| --- | --- | --- |
| rustboyadvance-ng | `1d7ff23a6adb42d803ebd393c3d27d94e4fea15e` | ARM execution, bus structure, native/WASM separation, multi-channel DMA request/repeat reference |
| mGBA | `c3c8e5e813f245028de118a56734e1dc0f35ce2a` | GBA display, timing, BIOS protection/open-bus, Flash, color effects, HBlank/VBlank DMA requests and arbitration |
| mGBA suite | `e6942030d25ffe3ba76c72b73a86da073ec857cc` | Slice 22 standalone OBJ comparison; inspected for Slice 24 blend cases |
| NanoBoyAdvance | `55b5cf0ae3d929582ac5bfd486558173502b8354` | cycle scheduling, display synchronization, color-effect ordering, rounding, hidden green blend precision, and visible-line HBlank DMA placement |
| SkyEmu | `01516d6798e3652b583e6a366085bb51c43b528d` | browser/native emulator integration |
| gba-tests | `a7113b67e63f83a9b321696ddd7042ccfad6c881` | ARM and GBA guest-test conventions; pinned save/flash64 and bios guests and macros |

These revisions identify the source snapshots used as references. Behavior is reimplemented independently in this project.
