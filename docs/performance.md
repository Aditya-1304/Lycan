# Performance and reference device

## Reference device

- Date:
- Arch Linux version:
- Desktop environment and session type:
- CPU:
Architecture:                            x86_64
CPU op-mode(s):                          32-bit, 64-bit
Address sizes:                           39 bits physical, 48 bits virtual
Byte Order:                              Little Endian
CPU(s):                                  16
On-line CPU(s) list:                     0-15
Vendor ID:                               GenuineIntel
Model name:                              13th Gen Intel(R) Core(TM) i7-13620H
CPU family:                              6
Model:                                   186
Thread(s) per core:                      2
Core(s) per socket:                      10
Socket(s):                               1
Stepping:                                2
Microcode version:                       0x6134
CPU(s) scaling MHz:                      38%
CPU max MHz:                             4900.0000
CPU min MHz:                             400.0000
BogoMIPS:                                5836.80
Flags:                                   fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush dts acpi mmx fxsr sse sse2 ss ht tm pbe syscall nx pdpe1gb rdtscp lm constant_tsc art arch_perfmon pebs bts rep_good nopl xtopology nonstop_tsc cpuid aperfmperf tsc_known_freq pni pclmulqdq dtes64 monitor ds_cpl vmx smx est tm2 ssse3 sdbg fma cx16 xtpr pdcm pcid sse4_1 sse4_2 x2apic movbe popcnt tsc_deadline_timer aes xsave avx f16c rdrand lahf_lm abm 3dnowprefetch cpuid_fault epb ssbd ibrs ibpb stibp ibrs_enhanced tpr_shadow flexpriority ept vpid ept_ad fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid rdseed adx smap clflushopt clwb intel_pt sha_ni xsaveopt xsavec xgetbv1 xsaves split_lock_detect user_shstk avx_vnni dtherm ida arat pln pts hwp hwp_notify hwp_act_window hwp_epp hwp_pkg_req hfi vnmi umip pku ospke waitpkg gfni vaes vpclmulqdq rdpid movdiri movdir64b fsrm md_clear serialize arch_lbr ibt flush_l1d arch_capabilities
Virtualization:                          VT-x
L1d cache:                               416 KiB (10 instances)
L1i cache:                               448 KiB (10 instances)
L2 cache:                                9.5 MiB (7 instances)
L3 cache:                                24 MiB (1 instance)
NUMA node(s):                            1
NUMA node0 CPU(s):                       0-15
Vulnerability Gather data sampling:      Not affected
Vulnerability Ghostwrite:                Not affected
Vulnerability Indirect target selection: Not affected
Vulnerability Itlb multihit:             Not affected
Vulnerability L1tf:                      Not affected
Vulnerability Mds:                       Not affected
Vulnerability Meltdown:                  Not affected
Vulnerability Mmio stale data:           Not affected
Vulnerability Old microcode:             Not affected
Vulnerability Reg file data sampling:    Mitigation; Clear Register File
Vulnerability Retbleed:                  Not affected
Vulnerability Spec rstack overflow:      Not affected
Vulnerability Spec store bypass:         Mitigation; Speculative Store Bypass disabled via prctl
Vulnerability Spectre v1:                Mitigation; usercopy/swapgs barriers and __user pointer sanitization
Vulnerability Spectre v2:                Mitigation; Enhanced / Automatic IBRS; IBPB conditional; PBRSB-eIBRS SW sequence; BHI BHI_DIS_S
Vulnerability Srbds:                     Not affected
Vulnerability Tsa:                       Not affected
Vulnerability Tsx async abort:           Not affected
Vulnerability Vmscape:                   Mitigation; IBPB before exit to userspace

- Display resolution, scale, and refresh rate:
Monitor eDP-1 (ID 0):
	1920x1080@60.00100 at 0x0
	description: Chimei Innolux Corporation 0x1552
	make: Chimei Innolux Corporation
	model: 0x1552
	physical size (mm): 340x190
	serial: 
	active workspace: 5 (5)
	special workspace: 0 ()
	reserved: 0 45 0 0
	scale: 1
	transform: 0
	focused: yes
	dpmsStatus: 1
	vrr: false
	solitary: 0
	solitaryBlockedBy: windowed mode,missing candidate
	activelyTearing: false
	tearingBlockedBy: next frame is not torn,user settings,missing candidate
	directScanoutTo: 0
	directScanoutBlockedBy: user settings,missing candidate
	disabled: false
	currentFormat: XRGB8888
	mirrorOf: none
	availableModes: 1920x1080@60.00Hz 
	colorManagementPreset: srgb
	sdrBrightness: 1
	sdrSaturation: 1
	sdrMinLuminance: 0.2
	sdrMaxLuminance: 80
	hardwareCursorsInUse: true

- Audio output device:
PipeWire 'pipewire-0' [1.6.8, aditya@aditya-arch, cookie:1171489013]
 └─ Clients:
        32. WirePlumber                         [1.6.8, aditya@aditya-arch, pid:1390]
        41. WirePlumber [client]                [1.6.8, aditya@aditya-arch, pid:1390]
        61. pipewire                            [1.6.8, aditya@aditya-arch, pid:1901]
        62. xdg-desktop-portal                  [1.6.8, aditya@aditya-arch, pid:1809]
        63. xdg-desktop-portal-hyprland         [1.6.8, aditya@aditya-arch, pid:2125]
        64. Blueman                             [1.6.8, aditya@aditya-arch, pid:1569]
        65. waybar                              [1.6.8, aditya@aditya-arch, pid:2283]
        66. waybar                              [1.6.8, aditya@aditya-arch, pid:2283]
        67. cava                                [1.6.8, aditya@aditya-arch, pid:2376]
        75. Google Chrome input                 [1.6.8, aditya@aditya-arch, pid:22981]
        88. Chromium input                      [1.6.8, aditya@aditya-arch, pid:1671486]
        93. Spotify                             [1.6.8, aditya@aditya-arch, pid:1671083]
        97. wpctl                               [1.6.8, aditya@aditya-arch, pid:1723802]

Audio
 ├─ Devices:
 │      42. Built-in Audio                      [alsa]
 │  
 ├─ Sinks:
 │  *   49. Built-in Audio Analog Stereo        [vol: 1.25]
 │  
 ├─ Sources:
 │  *   50. Built-in Audio Analog Stereo        [vol: 0.76]
 │  
 ├─ Filters:
 │  
 └─ Streams:
        68. cava                                                        
             69. input_FL        < ALC257 Analog:monitor_FL	[active]
             70. input_FR        < ALC257 Analog:monitor_FR	[active]
             71. monitor_FL     
             72. monitor_FR     
        89. Spotify                                                     
             76. output_FL       > ALC257 Analog:playback_FL	[paused]
             83. output_FR       > ALC257 Analog:playback_FR	[paused]

Video
 ├─ Devices:
 │      51. Integrated Camera                   [v4l2]
 │      52. Integrated Camera                   [v4l2]
 │  
 ├─ Sinks:
 │  
 ├─ Sources:
 │  *   59. Integrated Camera (V4L2)           
 │  
 ├─ Filters:
 │  
 └─ Streams:

Settings
 └─ Default Configured Devices:

## Toolchain and build settings

- rustc 1.98.1 (48a229cea 2026-09-01)
- binary: rustc
- commit-hash: 48a229ceaefd4985c50990b14116b6d856af0985
- commit-date: 2026-09-01
- host: x86_64-unknown-linux-gnu
- release: 1.98.1
- LLVM version: 22.1.8
- cargo 1.98.1 (797e8a9bc 2026-08-05)
- trunk 0.21.14
- Native build: release profile, opt-level 3, thin LTO
- Browser build: Trunk release build
- Tracing: disabled or not yet present

### Operating system
NAME="Arch Linux"
PRETTY_NAME="Arch Linux"
ID=arch
BUILD_ID=rolling
ANSI_COLOR="38;2;23;147;209"
HOME_URL="https://archlinux.org/"
DOCUMENTATION_URL="https://wiki.archlinux.org/"
SUPPORT_URL="https://bbs.archlinux.org/"
BUG_REPORT_URL="https://gitlab.archlinux.org/groups/archlinux/-/issues"
PRIVACY_POLICY_URL="https://terms.archlinux.org/docs/privacy-policy/"
LOGO=archlinux-logo

### Desktop session
XDG_SESSION_TYPE=wayland

### Browser versions
google-chrome-stable: Google Chrome 153.0.8010.47 
brave: Brave Browser 153.1.95.102 

## Benchmark conditions

- Power: AC connected
- Power profile: performance
- Browser extensions affecting test: none
- Native/WASM benchmark runs performed with the same conditions

## Slice measurements

Record measurements as the emulator gains runnable slices.
