#!/usr/bin/env bash
# provision-vm.sh — turn a Debian 12 VM into the Tunnriculum appliance.
#
# Can be run in TWO ways:
#   A) Directly INSIDE the Debian 12 VM:
#        git clone <repo> /home/lab/tunnriculum
#        cd /home/lab/tunnriculum
#        ./train/scripts/appliance/provision-vm.sh
#
#   B) From your HOST machine (pushes into the VM over SSH):
#        ./train/scripts/appliance/provision-vm.sh lab@<VM_IP>
#
# After provisioning, logging in as 'tunnriculum' (ssh tunnriculum@<VM_IP>,
# password: tunnriculum) opens the labctl Rust TUI directly.
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
TARGET=${1:-}

# --- Remote mode: push repo to VM over SSH, then run this script inside the VM ---
if [[ -n "$TARGET" ]]; then
  VM_HOST=${TARGET#*@}
  KEY="$HOME/.ssh/tunnriculum-lab"
  SSH_OPTS=(-o StrictHostKeyChecking=no -o ConnectTimeout=5)
  [ -f "$KEY" ] || KEY="$HOME/.ssh/debian-tunnel-lab"
  [ -f "$KEY" ] && SSH_OPTS+=(-i "$KEY")
  SSH="ssh ${SSH_OPTS[*]} $TARGET"

  echo ">>> Pushing Tunnriculum to $TARGET:/home/lab/tunnriculum/train/"
  $SSH 'command -v rsync >/dev/null || (sudo apt-get update -qq && sudo apt-get install -yqq rsync sudo >/dev/null)
        sudo mkdir -p /home/lab/tunnriculum/train
        sudo chown -R "$(id -un):$(id -gn)" /home/lab/tunnriculum'

  rsync -az --delete \
    --exclude '.git' \
    --exclude 'instances/*.env' \
    --exclude 'instances/*_key*' \
    --exclude 'labctl-rs/target' \
    --exclude 'dist' \
    --exclude '*.qcow2' \
    -e "ssh ${SSH_OPTS[*]}" \
    "$root/" "$TARGET:/home/lab/tunnriculum/train/"

  echo ">>> Running appliance provisioning inside $TARGET"
  $SSH '/home/lab/tunnriculum/train/scripts/appliance/provision-vm.sh'
  echo ">>> Done! Enter the TUI console with:"
  echo "    ssh tunnriculum@$VM_HOST    (password: tunnriculum, Settings PIN: 1337)"
  exit 0
fi

# --- Local mode (running inside the VM) ---
echo ">>> [1/4] Installing Docker and system packages"
if ! command -v docker >/dev/null || ! command -v rsync >/dev/null; then
  export DEBIAN_FRONTEND=noninteractive
  sudo apt-get update -qq
  sudo apt-get install -yqq docker.io docker-compose qemu-guest-agent sudo git rsync curl netcat-openbsd >/dev/null
  sudo systemctl enable --now docker qemu-guest-agent 2>/dev/null || true
fi

echo ">>> [2/4] Configuring 'lab' (admin) and 'tunnriculum' (TUI player) users"
id -u lab >/dev/null 2>&1 || sudo useradd -m -s /bin/bash lab
sudo usermod -aG docker lab
printf "lab ALL=(ALL) NOPASSWD:ALL\n" | sudo tee /etc/sudoers.d/lab >/dev/null
sudo chmod 440 /etc/sudoers.d/lab

# Ensure code lives at /home/lab/tunnriculum/train
if [[ "$root" != "/home/lab/tunnriculum/train" ]]; then
  sudo mkdir -p /home/lab/tunnriculum/train
  sudo rsync -a --delete --exclude 'labctl-rs/target' "$root/" /home/lab/tunnriculum/train/
fi
sudo chown -R lab:lab /home/lab/tunnriculum

id -u tunnriculum >/dev/null 2>&1 || sudo useradd -m -g operator -s /usr/local/bin/labctl tunnriculum
sudo usermod -s /usr/local/bin/labctl tunnriculum
echo "tunnriculum:tunnriculum" | sudo chpasswd
printf "tunnriculum ALL=(lab) NOPASSWD: /home/lab/tunnriculum/train/scripts/lab.sh *\n" | sudo tee /etc/sudoers.d/tunnriculum >/dev/null
sudo chmod 440 /etc/sudoers.d/tunnriculum

sudo mkdir -p /home/tunnriculum/.ssh
if [ -f "$HOME/.ssh/authorized_keys" ]; then
  sudo cp "$HOME/.ssh/authorized_keys" /home/tunnriculum/.ssh/authorized_keys
  sudo chmod 600 /home/tunnriculum/.ssh/authorized_keys
elif [ -f /home/lab/.ssh/authorized_keys ]; then
  sudo cp /home/lab/.ssh/authorized_keys /home/tunnriculum/.ssh/authorized_keys
  sudo chmod 600 /home/tunnriculum/.ssh/authorized_keys
fi
sudo chmod 700 /home/tunnriculum/.ssh
sudo chown -R tunnriculum:operator /home/tunnriculum/.ssh

echo ">>> [3/4] Building and installing /usr/local/bin/labctl (Rust TUI)"
sudo docker run --rm --network=host \
  -v /home/lab/tunnriculum/train/labctl-rs:/labctl -w /labctl \
  docker.io/library/rust:alpine \
  cargo build --release --target x86_64-unknown-linux-musl

BIN="/home/lab/tunnriculum/train/labctl-rs/target/x86_64-unknown-linux-musl/release/labctl"
[ -x "$BIN" ] || { echo "!! labctl build failed" >&2; exit 1; }
LABCTL_SELFTEST=1 "$BIN" >/dev/null || { echo "!! labctl selftest failed" >&2; exit 1; }

sudo install -m 755 "$BIN" /usr/local/bin/labctl
sudo rm -rf /home/lab/tunnriculum/train/labctl-rs/target
grep -qx /usr/local/bin/labctl /etc/shells || echo /usr/local/bin/labctl | sudo tee -a /etc/shells >/dev/null

echo ">>> [4/4] Enabling systemd services and starting the 3-zone lab"
cd /home/lab/tunnriculum/train
sudo cp scripts/appliance/tunnriculum-firstboot.service \
       scripts/appliance/tunnriculum-lab.service \
       scripts/appliance/tunnriculum-banner.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable tunnriculum-firstboot.service tunnriculum-lab.service tunnriculum-banner.service >/dev/null 2>&1
sudo systemctl start tunnriculum-firstboot.service tunnriculum-lab.service

vm_ip=$(ip -4 -o addr show scope global | awk '{print $4}' | cut -d/ -f1 | head -1)
echo ">>> Provisioning complete!"
echo "    Enter the TUI console: ssh tunnriculum@${vm_ip:-<VM_IP>}  (password: tunnriculum, PIN: 1337)"
