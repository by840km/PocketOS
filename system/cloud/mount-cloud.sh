#!/bin/sh
# Development template. Run in the guest, never in Windows.
set -eu
: "${HOME:?HOME is required}"
config="$HOME/.config/pocketos/rclone.conf"
cache="$HOME/.local/share/pocketos/vfs-cache"
mountpoint="$HOME/Cloud"
if [ ! -s "$config" ]; then
    echo "Configure pocketcloud remote first: rclone config --config $config" >&2
    exit 1
fi
umask 077
mkdir -p "$cache" "$mountpoint"
# A soft cache budget: dirty/open files can exceed it. Do not delete pending data.
# Encryption, telemetry and a shutdown coordinator are not implemented yet.
exec rclone mount pocketcloud: "$mountpoint" \
    --config "$config" \
    --cache-dir "$cache" \
    --vfs-cache-mode full \
    --vfs-cache-max-size 2G \
    --vfs-write-back 5s \
    --dir-cache-time 30s
