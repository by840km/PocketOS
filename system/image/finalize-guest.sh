#!/bin/sh
# Run only in the completed development guest, before distributing its disk.
set -eu
apt-get clean
systemctl disable ssh.service ssh.socket
systemctl mask ssh.service ssh.socket
touch /etc/cloud/cloud-init.disabled
rm -f /etc/ssh/ssh_host_*key /etc/ssh/ssh_host_*key.pub /root/.bash_history
rm -f /root/.ssh/authorized_keys /root/pocketos-build.sh /root/pocketos-profile.yaml /etc/dpkg/dpkg.cfg.d/99-pocketos-build
rm -rf /var/lib/cloud
rm -f /var/log/cloud-init.log /var/log/cloud-init-output.log
sync
