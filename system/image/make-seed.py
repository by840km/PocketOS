"""Create NoCloud seed. Dependency: pycdlib. No credentials in this dev profile."""
import io
import base64
import pathlib
import sys
import pycdlib
source = pathlib.Path(sys.argv[1])
output = pathlib.Path(sys.argv[2])
iso = pycdlib.PyCdlib()
iso.new(interchange_level=3, joliet=3, rock_ridge="1.09", vol_ident="cidata")
for name in ("user-data", "meta-data"):
    content = (source / name).read_text(encoding="utf-8-sig")
    if name == 'user-data':
        files = [(source/'configure-guest.sh', '/root/configure-guest.sh')]
        files += [(p, '/root/plasma/'+p.name) for p in sorted((source.parent/'plasma').iterdir()) if p.is_file()]
        entries = []
        for path, target in files:
            encoded = base64.b64encode(path.read_bytes()).decode()
            entries.append(f"  - path: {target}\n    encoding: b64\n    permissions: '0644'\n    content: {encoded}")
        content = content.replace('# POCKETOS_PLASMA_FILES', '\n'.join(entries))
    data = content.encode('utf-8')
    iso.add_fp(io.BytesIO(data), len(data), iso_path="/" + name.upper().replace("-", "_") + ";1", rr_name=name, joliet_path="/" + name)
iso.write(str(output))
iso.close()
