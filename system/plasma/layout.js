// PocketOS desktop: only stock Plasma widgets, no downloaded executable plugins.
var existing = panels();
for (var i = 0; i < existing.length; i++) existing[i].remove();
var p = new Panel;
p.location = 'bottom';
p.height = 48;
p.floating = true;
var menu = p.addWidget('org.kde.plasma.kickoff');
menu.currentConfigGroup = ['General'];
menu.writeConfig('icon', 'start-here-kde');
var tasks = p.addWidget('org.kde.plasma.icontasks');
tasks.currentConfigGroup = ['General'];
tasks.writeConfig('launchers', ['applications:org.kde.dolphin.desktop','applications:org.kde.konsole.desktop','applications:org.kde.kate.desktop']);
p.addWidget('org.kde.plasma.marginsseparator');
p.addWidget('org.kde.plasma.systemtray');
var clock = p.addWidget('org.kde.plasma.digitalclock');
clock.currentConfigGroup = ['Appearance'];
clock.writeConfig('showSeconds', 'Never');
p.addWidget('org.kde.plasma.showdesktop');
var ds = desktops();
for (var j = 0; j < ds.length; j++) {
    ds[j].wallpaperPlugin = 'org.kde.image';
    ds[j].currentConfigGroup = ['Wallpaper', 'org.kde.image', 'General'];
    ds[j].writeConfig('Image', 'file:///usr/share/wallpapers/PocketOS/contents/images/1920x1080.svg');
}

print('POCKETOS_LAYOUT_READY panels=' + panels().length + ' floating=' + p.floating);
