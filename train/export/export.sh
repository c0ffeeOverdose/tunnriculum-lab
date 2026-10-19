#!/usr/bin/env bash
# export.sh — build a distributable Tunnriculum appliance pack.
#
# One command produces dist/tunnriculum-<date>/ with everything the recipient
# needs to run the lab: the compacted VM image, a fresh SSH admin keypair
# (the dev key is removed from the image), and the WireGuard peer config.
#
# Pipeline:
#   1. ensure the VM is provisioned and running (provision-vm.sh is idempotent)
#   2. wipe the current instance (env + volumes) for a clean, fresh export
#   3. generate a new SSH keypair for this appliance; replace the dev key
#   4. reboot -> firstboot creates fresh flags/passwords and starts the lab
#   5. wait for smoke to pass, fetch the (new) WireGuard peer config
#   6. compact the disk (build-image.sh) and assemble the pack
#
# Usage: train/export/export.sh
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
train=$(cd "$here/.." && pwd)
repo=$(cd "$train/.." && pwd)
KEY="$HOME/.ssh/tunnriculum-lab"
VM="lab@192.168.122.30"
VMSSH=(ssh -i "$KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=5 "$VM")
STAMP=$(date +%Y%m%d-%H%M)
PACK="$repo/dist/tunnriculum-$STAMP"

mkdir -p "$PACK"

echo ">>> [1/6] ensuring VM is provisioned and running"
"$train/scripts/vm-up.sh" up >/dev/null 2>&1 || \
  virsh --connect qemu:///system start tunnriculum-lab >/dev/null 2>&1 || true
for i in $(seq 1 24); do
  sleep 5
  "${VMSSH[@]}" 'echo up' >/dev/null 2>&1 && break
  [ "$i" = 24 ] && { echo "!! VM unreachable" >&2; exit 1; }
done
"$train/scripts/appliance/provision-vm.sh" >/dev/null

echo ">>> [2/6] wiping instance for a clean export"
"${VMSSH[@]}" 'set -e
  cd /home/lab/tunnriculum/train
  ./scripts/lab.sh down >/dev/null 2>&1 || true
  rm -f instances/tunnriculum.env
'

echo ">>> [3/6] generating fresh admin SSH keypair"
ssh-keygen -t ed25519 -N "" -C "tunnriculum-$STAMP" -f "$PACK/admin_ed25519" -q
"${VMSSH[@]}" "set -e
  install -m 600 -o lab -g lab /dev/null /home/lab/.ssh/authorized_keys
  cat > /home/lab/.ssh/authorized_keys <<'KEYEOF'
$(cat "$PACK/admin_ed25519.pub")
KEYEOF
  sudo install -m 600 -o tunnriculum -g operator /dev/null /home/tunnriculum/.ssh/authorized_keys
  sudo tee /home/tunnriculum/.ssh/authorized_keys >/dev/null <<'KEYEOF'
$(cat "$PACK/admin_ed25519.pub")
KEYEOF
"

echo ">>> [4/6] rebooting — firstboot will create fresh flags and start the lab"
virsh --connect qemu:///system reboot tunnriculum-lab >/dev/null
ok=0
for i in $(seq 1 36); do
  sleep 5
  if "${VMSSH[@]}" 'test -f /home/lab/tunnriculum/train/instances/tunnriculum.env && docker ps --format "{{.Names}}" 2>/dev/null | grep -q tunnriculum-' >/dev/null 2>&1; then ok=1; break; fi
done
[ "$ok" = 1 ] || { echo "!! firstboot did not come up" >&2; exit 1; }

echo ">>> [5/6] smoke + peer config"
for i in $(seq 1 12); do
  "${VMSSH[@]}" '/home/lab/tunnriculum/train/scripts/lab.sh smoke' >/dev/null 2>&1 && break
  sleep 5
  [ "$i" = 12 ] && { echo "!! smoke failed" >&2; exit 1; }
done
"${VMSSH[@]}" '/home/lab/tunnriculum/train/scripts/lab.sh peer' > "$PACK/peer.conf"

echo ">>> [6/6] compacting image"
"$train/scripts/appliance/build-image.sh" >/dev/null
mv "$train/dist/tunnriculum-appliance.qcow2" "$PACK/tunnriculum-appliance.qcow2"
mv "$train/dist/tunnriculum-appliance.xml" "$PACK/tunnriculum-appliance.xml"

cat > "$PACK/README.txt" <<EOF
Tunnriculum — double-pivot network lab (appliance)

Contents
  tunnriculum-appliance.qcow2   VM disk image (import with the XML below)
  tunnriculum-appliance.xml     libvirt domain definition
  admin_ed25519 / .pub          SSH admin key (user: lab)
  peer.conf                     WireGuard peer config for this instance

Import (KVM host)
  cp tunnriculum-appliance.qcow2 /var/lib/libvirt/images/
  virsh define tunnriculum-appliance.xml
  virsh start tunnriculum-appliance

After boot (~1 min) the lab starts itself with fresh flags.
  Lab address : 192.168.122.30
  Admin SSH   : ssh -i admin_ed25519 lab@192.168.122.30
  TUI         : ssh -i admin_ed25519 tunnriculum@192.168.122.30  (PIN 1337)

Player connection
  sudo wg-quick up peer.conf     # sees only 10.100.100.0/24 (DMZ)

Regenerate a player peer config at any time: TUI -> Settings -> Peer config.
Reset the instance (new flags): remove instances/tunnriculum.env inside the VM and reboot.
EOF

ls -lh "$PACK"
echo ">>> pack ready: $PACK"
