# PocketOS direct boot / RAM root

The base QCOW2 is read-only. OverlayFS upper/work live in a 1 GiB tmpfs; all session changes disappear at shutdown. Persistent profiles and cloud synchronization are not implemented.

Build with Python 3: `python build.py installer-initrd.gz kernel-data.tar output-directory [debug-public-key]`. Inputs are verified against `sources.lock.json`. Extract the pinned kernel .deb to obtain data.tar. The builder includes bootstrap tools/shared libraries and only the boot module dependency closure (including ext4 CRC32C providers), rather than two complete kernel module trees. Modules matching the kernel must also be installed in the guest root. Copy output vmlinuz/initrd.img beside system/pocketos.qcow2 after successful completion.

Normal release images contain no debug key. The writable build branch requires BOTH an injected public key and the explicit kernel parameter `pocketos.build=rw`; it is only for a dedicated writable build overlay, never a normal USB launch. Debug keys and host-forwarded SSH are not included in the portable release.

At boot, /init mounts the read-only ext4 lower, RAM upper and OverlayFS root, preserves backing mounts and uses switch_root. fstab/growfs overrides are applied inside RAM. A systemd reporter sends kernel/root/desktop events over ttyS0. Readiness requires plasmashell on the user bus and completion of the PocketOS desktop layout. The Windows loader hides early boot until guest and fullscreen window are ready.

The minimized initrd is about 9.2 MiB versus about 135 MiB in 0.3. This size reduction is not a full USB boot benchmark; see docs/UPDATE-0.4.md for measured results.
