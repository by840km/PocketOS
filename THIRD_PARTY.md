# Third-party components

The MIT license in this repository covers original PocketOS source only.
Debian/Linux, KDE, QEMU, their libraries and Catppuccin retain their own licenses.
Catppuccin's MIT notice is included in system/plasma/LICENSE.catppuccin.
The runtime retains QEMU's bundled notices and Debian package copyright files under /usr/share/doc inside the guest.

Debian image provenance and checksums: system/image/sources.lock.json.
Kernel/installer provenance: system/ramroot/sources.lock.json.
Installed Debian package versions: system/image/packages-beta.txt.
Debian corresponding source packages: https://sources.debian.org/ and https://snapshot.debian.org/ (use the recorded package versions).
QEMU Windows distribution: https://qemu.weilnetz.de/w64/ ; this build reports 11.1.0 (v11.1.0-12130-ge470268ff4).
QEMU upstream source: https://gitlab.com/qemu-project/qemu ; Windows producer: https://github.com/stweil/qemu .
Build recipes and modifications to the guest are included under system/.
