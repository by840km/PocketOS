# USB installer

Windows GUI written in Rust with native Windows controls. Lists only USB disks,
excluding system/boot disks, and only NTFS/exFAT volumes. Does not format drives.
Refuses to overwrite an existing PocketOS directory. Checks drive identity and
available space, downloads SHA256-verified chunks, rejects archive traversal and
symlinks, and stages files before renaming the completed PocketOS directory.

The usb-beta branch contains manifest.json and 48 MB archive parts. Main contains
sources. GitHub Releases contains the installer and the complete portable archive.
No account or token is needed to download a public beta.

Build: set POCKETOS_MANIFEST_URL to the published raw manifest URL, then run
cargo build --release -p pocketos-installer. Never distribute a build using OWNER.

Beta limits: no resumable downloads yet; interrupted installation leaves
.PocketOS-installing for manual cleanup. USB removal stops installation. Existing
files outside the staging directory are never removed. Runtime sessions are volatile.
