"""Package portable USB files, splitting into ordinary-Git-sized chunks.
Requires --owner before creating the upload manifest. No secrets/backups are included.
"""
import argparse,pathlib,zipfile,hashlib,json
p=argparse.ArgumentParser();p.add_argument('--owner',required=True);p.add_argument('--runtime',default='D:/PocketOS');p.add_argument('--out',required=True);a=p.parse_args()
r=pathlib.Path(a.runtime);out=pathlib.Path(a.out);out.mkdir(parents=True,exist_ok=True)
archive=out/'PocketOS-0.4.0-beta.1-usb.zip'
with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_STORED,allowZip64=True) as z:
 for relative in ['PocketOS.exe','pocketos-core.exe','qemu','system']:
  entry=r/relative
  files=entry.rglob('*') if entry.is_dir() else [entry]
  for f in files:
   if f.is_file():z.write(f,'PocketOS/'+f.relative_to(r).as_posix())
parts=[]
with archive.open('rb') as stream:
 i=0
 while data:=stream.read(48_000_000):
  name=f'usb-{i:03d}.part';(out/name).write_bytes(data)
  parts.append(dict(name=name,size=len(data),sha256=hashlib.sha256(data).hexdigest(),url=f'https://raw.githubusercontent.com/{a.owner}/PocketOS/usb-beta/{name}'));i+=1
(out/'manifest.json').write_text(json.dumps(dict(version='0.4.0-beta.1',required_free=5_000_000_000,parts=parts),indent=2),encoding='utf-8')
print('Archive and verified parts ready',len(parts),archive.stat().st_size)
