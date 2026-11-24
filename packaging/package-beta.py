"""Package portable USB files, as a standalone ZIP.
No installer or manifest is required. No secrets/backups are included.
"""
import argparse,pathlib,zipfile,hashlib,json
p=argparse.ArgumentParser();p.add_argument('--runtime',default='D:/PocketOS');p.add_argument('--out',required=True);p.add_argument('--image');p.add_argument('--qemu');a=p.parse_args()
r=pathlib.Path(a.runtime);out=pathlib.Path(a.out);out.mkdir(parents=True,exist_ok=True)
archive=out/'PocketOS-0.4.0-beta.1-usb.zip'
components=[(r/'PocketOS.exe','PocketOS/PocketOS.exe'),(r/'pocketos-core.exe','PocketOS/pocketos-core.exe')]
components += [(r/'system'/n,'PocketOS/system/'+n) for n in ['vmlinuz','initrd.img']]
components += [(pathlib.Path(a.image) if a.image else r/'system/pocketos.qcow2','PocketOS/system/pocketos.qcow2')]
qemu=pathlib.Path(a.qemu) if a.qemu else r/'qemu'
if not (qemu/'qemu-system-x86_64.exe').is_file():raise ValueError('QEMU missing')
components += [(f,'PocketOS/qemu/'+f.relative_to(qemu).as_posix()) for f in qemu.rglob('*') if f.is_file() and '$PLUGINSDIR' not in f.parts and (f.suffix.lower()!='.exe' or f.name in ['qemu-system-x86_64.exe','qemu-system-x86_64w.exe','qemu-img.exe'])]
for f,name in components:
 if not f.is_file() :raise ValueError('Missing runtime file: '+str(f))
with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=1,allowZip64=True) as z:
 for f,name in components:z.write(f,name)
if archive.stat().st_size >= 2_000_000_000:raise ValueError('Archive exceeds release/push size budget')
print('Portable ZIP ready',archive,archive.stat().st_size)
