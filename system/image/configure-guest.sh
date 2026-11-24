#!/bin/bash
set -euxo pipefail
export DEBIAN_FRONTEND=noninteractive
apt-get -o DPkg::Lock::Timeout=120 install -y --no-install-recommends kde-plasma-desktop kwin-x11 sddm konsole dolphin kate plasma-nm plasma-pa qdbus-qt6 network-manager pipewire-audio firefox-esr mesa-utils xdg-desktop-portal-kde papirus-icon-theme fonts-noto-core xserver-xorg-core xserver-xorg-input-libinput rclone fuse3 spice-vdagent libglib2.0-bin
# make-seed.py supplies the pinned theme/config files in this build-only path.
sh /root/plasma/configure.sh
mkdir -p /home/pocket/Cloud
chown pocket:pocket /home/pocket/Cloud
dpkg-query -W > /var/log/pocketos-packages.txt
touch /var/lib/pocketos-image-ready
