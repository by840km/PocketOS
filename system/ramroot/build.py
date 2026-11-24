"""Build a small direct-boot initrd from verified Debian installer and kernel inputs.
Usage: build.py installer-initrd.gz kernel-data.tar output-dir [debug-public-key]
The matching modules.dep is pinned alongside this builder; no network is used.
"""
import gzip, io, pathlib, stat, sys, tarfile, hashlib, json

HERE = pathlib.Path(__file__).parent
base, deb, out = map(pathlib.Path, sys.argv[1:4])
out.mkdir(parents=True, exist_ok=True)
lock = json.loads((HERE/'sources.lock.json').read_text())
for source, key in ((base, 'installer-initrd'), (deb, 'kernel-data'), (HERE/'modules.dep', 'modules.dep')):
    if hashlib.sha256(source.read_bytes()).hexdigest() != lock[key]['sha256']:
        raise ValueError('Source SHA256 mismatch: '+key)
archive = io.BytesIO()
inode = 1
seen = set()

def entry(name, data, mode):
    global inode
    if name in seen:
        return
    for parent in reversed(pathlib.PurePosixPath(name).parents):
        if str(parent) not in ('.', ''):
            entry(str(parent), b'', stat.S_IFDIR | 0o755)
    seen.add(name)
    encoded = name.encode() + b'\0'
    fields = [inode, mode, 0, 0, 1, 0, len(data), 0, 0, 0, 0, len(encoded), 0]
    inode += 1
    archive.write(b'070701' + b''.join(f'{v:08x}'.encode() for v in fields))
    archive.write(encoded)
    archive.write(b'\0' * (-archive.tell() % 4))
    archive.write(data)
    archive.write(b'\0' * (-archive.tell() % 4))

# Keep only bootstrap tools and their shared libraries, not the installer UI,
# old kernel modules, network tools, package database or localization payload.
blob = gzip.decompress(base.read_bytes())
offset = 0
while offset + 110 <= len(blob):
    if blob[offset:offset+6] not in (b'070701', b'070702'):
        raise ValueError('Invalid installer newc archive')
    h = [int(blob[offset+6+i*8:offset+14+i*8], 16) for i in range(13)]
    start = offset + 110
    name = blob[start:start+h[11]-1].decode().removeprefix('./')
    pos = (start + h[11] + 3) & ~3
    data = blob[pos:pos+h[6]]
    offset = (pos + h[6] + 3) & ~3
    if name == 'TRAILER!!!':
        break
    mode = h[1]
    library = name.startswith(('usr/lib/', 'usr/lib64/')) and not name.startswith(('usr/lib/modules/', 'usr/lib/udev/', 'usr/lib/debian-installer/', 'usr/lib/ossl-modules/'))
    tool = name in ('usr/bin/busybox', 'usr/bin/kmod')
    link = stat.S_ISLNK(mode) and (name in ('bin','sbin','lib','lib64') or name.startswith(('usr/bin/', 'usr/sbin/')))
    if library or tool or link:
        entry(name, data, mode)
for name in ('dev','proc','sys','run','tmp','etc','newroot','usr/sbin'):
    entry(name, b'', stat.S_IFDIR | 0o755)

# Resolve only the modules required to mount the guest disk and overlay root.
deps = {}
for line in (HERE/'modules.dep').read_text().splitlines():
    name, rest = line.split(':', 1)
    deps[name] = rest.split()
with tarfile.open(deb) as tar:
    builtin_member = next(m for m in tar.getmembers() if m.name.endswith('/modules.builtin'))
    builtin_names = {pathlib.PurePosixPath(n).name.split('.')[0].replace('-', '_') for n in tar.extractfile(builtin_member).read().decode().splitlines()}
wanted = set()
def include(name):
    if name in wanted:
        return
    wanted.add(name)
    for dependency in deps[name]:
        include(dependency)
for module in ('virtio_pci', 'virtio_blk', 'virtio_rng', 'ext4', 'overlay', 'crc32c_generic', 'crc32c_intel'):
    matches = [n for n in deps if pathlib.PurePosixPath(n).name.split('.')[0].replace('-', '_') == module]
    if module in builtin_names:
        continue
    if len(matches) != 1:
        raise ValueError('Missing or ambiguous module: '+module)
    include(matches[0])
with tarfile.open(deb) as tar:
    for item in tar:
        name = item.name.removeprefix('./')
        if name.startswith('boot/vmlinuz-'):
            (out/'vmlinuz').write_bytes(tar.extractfile(item).read())
        name = name.removeprefix('usr/')
        if not name.startswith('lib/modules/'):
            continue
        prefix, sep, relative = name.partition('/kernel/')
        kernel_path = 'kernel/'+relative if sep else ''
        if kernel_path in wanted or name.endswith(('/modules.builtin', '/modules.builtin.modinfo', '/modules.order')):
            if item.isfile():
                entry('usr/'+name, tar.extractfile(item).read(), stat.S_IFREG | item.mode)
init = (HERE/'init').read_text(encoding='utf-8-sig').replace('\r\n','\n').encode()
if len(sys.argv) > 4:
    key = pathlib.Path(sys.argv[4]).read_bytes()
    if not key.startswith((b'ssh-ed25519 ', b'ssh-rsa ')):
        raise ValueError('Debug key must be a PUBLIC SSH key')
    entry('pocketos-debug-key', key, stat.S_IFREG | 0o600)
entry('pocketos-plasma-layout', (HERE.parent/'plasma/apply-layout').read_bytes(), stat.S_IFREG | 0o755)
entry('init', init, stat.S_IFREG | 0o755)
entry('TRAILER!!!', b'', 0)
image = gzip.compress(archive.getvalue(), compresslevel=6, mtime=0)
(out/'initrd.img').write_bytes(image)
print(f'Built {len(wanted)} boot modules, initrd {len(image)/1048576:.1f} MiB: {out}')


