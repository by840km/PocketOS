# PocketOS 0.4 beta

KDE Plasma 6.3.6 replaces GNOME. Catppuccin Mocha Blue colors, native Breeze decorations, Papirus Dark icons and the PocketOS floating panel/wallpaper are included. Theme source/version/license are pinned in system/plasma.

The launcher disables extra QEMU monitor/parallel consoles and checks the actual desktop window bounds in physical pixels. One USB validation run reported a single 2560x1440 window and desktop readiness after 33.4 seconds. This was a warm run after copying/verifying files, not a cold USB benchmark.

The direct-boot initrd is approximately 9.2 MiB (formerly 128.9 MiB). The read-only zstd QCOW2 base is 1,687,945,216 bytes. The session still uses a 1 GiB RAM upper layer; changes disappear at shutdown. VirGL on RX 570, Plasma/KWin, NetworkManager and RAM overlay boot were validated. FPS and other PCs remain unmeasured.

Distribution: one portable ZIP in GitHub Releases. Extract to NTFS/exFAT USB and run PocketOS/PocketOS.exe from Windows. No installer or formatting.
