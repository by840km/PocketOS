#!/bin/sh
set -eu
# Run in the writable BUILD guest; normal USB sessions never modify the base.
install -d /usr/share/color-schemes /usr/share/pocketos/plasma /usr/share/wallpapers/PocketOS/contents/images /etc/sddm.conf.d /home/pocket/.config/autostart
install -m 644 /root/plasma/CatppuccinMochaBlue.colors /usr/share/color-schemes/
install -m 644 /root/plasma/LICENSE.catppuccin /usr/share/pocketos/plasma/
install -m 644 /root/plasma/layout.js /usr/share/pocketos/plasma/
install -m 644 /root/plasma/wallpaper.svg /usr/share/wallpapers/PocketOS/contents/images/1920x1080.svg
install -m 755 /root/plasma/apply-layout /usr/local/bin/pocketos-plasma-layout
cat > /etc/sddm.conf.d/pocketos.conf <<'EOF'
[Autologin]
User=pocket
Session=plasmax11.desktop
Relogin=true
[General]
DisplayServer=x11
EOF
cat > /home/pocket/.config/kdeglobals <<'EOF'
[General]
Name=PocketOS
font=Noto Sans,10,-1,5,50,0,0,0,0,0
fixed=DejaVu Sans Mono,10,-1,5,50,0,0,0,0,0
[KDE]
widgetStyle=Breeze
AnimationDurationFactor=0.5
SingleClick=false
[Icons]
Theme=Papirus-Dark
EOF
cat > /home/pocket/.config/kwinrc <<'EOF'
[Compositing]
Backend=OpenGL
GLCore=true
[Plugins]
blurEnabled=false
contrastEnabled=false
wobblywindowsEnabled=false
[org.kde.kdecoration2]
library=org.kde.breeze
BorderSize=Normal
ButtonsOnLeft=
ButtonsOnRight=IAX
EOF
cat > /home/pocket/.config/plasmarc <<'EOF'
[Theme]
name=default
EOF
cat > /home/pocket/.config/ksplashrc <<'EOF'
[KSplash]
Engine=none
Theme=None
EOF
cat > /home/pocket/.config/baloofilerc <<'EOF'
[Basic Settings]
Indexing-Enabled=false
EOF
cat > /home/pocket/.config/ksmserverrc <<'EOF'
[General]
loginMode=emptySession
EOF
cat > /home/pocket/.config/powermanagementprofilesrc <<'EOF'
[AC][SuspendSession]
idleTime=0
suspendType=0
EOF
cat > /home/pocket/.config/kscreenlockerrc <<'EOF'
[Daemon]
Autolock=false
LockOnResume=false
EOF
cat > /home/pocket/.config/autostart/pocketos-layout.desktop <<'EOF'
[Desktop Entry]
Type=Application
Name=PocketOS desktop layout
Exec=/usr/local/bin/pocketos-plasma-layout
OnlyShowIn=KDE;
EOF
chown -R pocket:pocket /home/pocket/.config
runuser -u pocket -- env QT_QPA_PLATFORM=offscreen plasma-apply-colorscheme CatppuccinMochaBlue
runuser -u pocket -- kwriteconfig6 --file kdeglobals --group General --key ColorScheme CatppuccinMochaBlue
# Replace cloud-init's networkd renderer with one portable NetworkManager profile.
rm -f /etc/netplan/50-cloud-init.yaml
mkdir -p /etc/netplan /etc/NetworkManager/system-connections
cat > /etc/netplan/90-pocketos.yaml <<'EOF'
network:
  version: 2
  renderer: NetworkManager
EOF
chmod 600 /etc/netplan/90-pocketos.yaml
cat > /etc/NetworkManager/system-connections/pocketos.nmconnection <<'EOF'
[connection]
id=PocketOS Ethernet
uuid=22623f02-2486-4e59-9d57-aac714e20e29
type=ethernet
autoconnect=true
[ethernet]
[ipv4]
method=auto
[ipv6]
method=auto
EOF
chmod 600 /etc/NetworkManager/system-connections/pocketos.nmconnection
netplan generate
systemctl disable gdm3.service systemd-networkd.service systemd-networkd.socket systemd-networkd-wait-online.service || true
systemctl mask systemd-networkd.service systemd-networkd.socket NetworkManager-wait-online.service systemd-networkd-wait-online.service apt-daily.timer apt-daily-upgrade.timer
usermod -aG netdev pocket
systemctl enable NetworkManager.service
printf '/usr/bin/sddm\n' > /etc/X11/default-display-manager
ln -sf /usr/lib/systemd/system/sddm.service /etc/systemd/system/display-manager.service
systemctl set-default graphical.target
# User keys are generated only for isolated diagnostics, never part of release.
