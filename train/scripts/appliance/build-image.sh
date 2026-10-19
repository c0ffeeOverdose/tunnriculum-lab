#!/usr/bin/env bash
# build-image.sh — export the provisioned VM as a distributable appliance image.
#
# Shuts the VM down cleanly, downloads the disk via libvirt (no host sudo
# needed), compacts it, and writes:
#   dist/tunnriculum-appliance.qcow2   (compressed, ~1-2 GB)
#   dist/tunnriculum-appliance.xml     (libvirt domain XML for virsh define)
#
# Usage: scripts/appliance/build-image.sh [domain]   # default tunnriculum-lab
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
DOMAIN=${1:-tunnriculum-lab}
DISK="/var/lib/libvirt/images/$DOMAIN.qcow2"
OUT="$root/dist"
mkdir -p "$OUT"

echo ">>> shutting down $DOMAIN"
virsh --connect qemu:///system shutdown "$DOMAIN" 2>/dev/null || true
for i in $(seq 1 24); do
  sleep 5
  virsh --connect qemu:///system dominfo "$DOMAIN" 2>/dev/null | grep -q "State:.*shut off" && break
  [ "$i" = 24 ] && { echo "!! VM did not shut down" >&2; exit 1; }
done

echo ">>> downloading disk (sparse)"
virsh --connect qemu:///system vol-download --pool default \
  --vol "$DOMAIN.qcow2" --file /tmp/tunnriculum-raw.qcow2 --sparse

echo ">>> compacting (qcow2 -c)"
qemu-img convert -O qcow2 -c /tmp/tunnriculum-raw.qcow2 "$OUT/tunnriculum-appliance.qcow2"
rm -f /tmp/tunnriculum-raw.qcow2

echo ">>> writing domain XML"
virsh --connect qemu:///system dumpxml "$DOMAIN" | python3 - "$OUT" <<'EOF'
import re, sys
out = sys.argv[1]
xml = sys.stdin.read()
xml = xml.replace("/var/lib/libvirt/images/tunnriculum-lab.qcow2",
                  "/var/lib/libvirt/images/tunnriculum-appliance.qcow2")
xml = re.sub(r'<uuid>.*?</uuid>\n\s*', '', xml)          # new UUID at define time
xml = re.sub(r"<mac address='.*?'/>", "<mac address='52:54:00:7b:4b:da'/>", xml)
open(f"{out}/tunnriculum-appliance.xml", "w").write(xml)
EOF

echo ">>> restart the VM?"
virsh --connect qemu:///system start "$DOMAIN" 2>/dev/null || true

ls -lh "$OUT"
echo ">>> done. Import on another KVM host:"
echo "    cp tunnriculum-appliance.qcow2 /var/lib/libvirt/images/"
echo "    virsh define tunnriculum-appliance.xml && virsh start tunnriculum-appliance"
