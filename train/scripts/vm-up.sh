#!/usr/bin/env bash
# vm-up.sh — manage tunnriculum-lab VM (replaces lab-host dind)
# The VM runs docker directly; lab services run inside it.
set -euo pipefail

NAME=tunnriculum-lab
# VM address: dynamic via qemu-guest-agent (falls back to the libvirt default
# static lease; override with TUNN_VM_IP). Works regardless of hypervisor
# network config — DHCP or static, libvirt or VirtualBox-imported disk.
vm_ip() {
  if [[ -n ${TUNN_VM_IP:-} ]]; then
    echo "$TUNN_VM_IP"; return
  fi
  local mac out
  mac=$(virsh --connect qemu:///system dumpxml "$NAME" 2>/dev/null | grep -oE "52:54:00:[0-9a-f:]{8}|0a:00:27:[0-9a-f:]{8}" | head -1)
  out=$(virsh --connect qemu:///system domifaddr "$NAME" --source agent 2>/dev/null \
    | grep "$mac" | grep -oE '([0-9]{1,3}\.){3}[0-9]{1,3}' | head -1)
  echo "${out:-192.168.122.30}"
}
VM_IP=$(vm_ip)
VM_USER=lab
SSH_KEY="$HOME/.ssh/tunnriculum-lab"
SSH_OPTS="-i $SSH_KEY -o StrictHostKeyChecking=no -o ConnectTimeout=3 -o BatchMode=yes"

cmd=${1:-up}

vm_ssh() {
  ssh $SSH_OPTS "$VM_USER@$VM_IP" "$@"
}

wait_vm() {
  echo "waiting for $NAME..." >&2
  for i in $(seq 1 30); do
    local state
    state=$(virsh --connect qemu:///system dominfo "$NAME" 2>/dev/null | grep "State:")
    if [[ $state == *running* ]]; then
      VM_IP=$(vm_ip)  # re-resolve: DHCP lease may change across boots
      if ssh $SSH_OPTS "$VM_USER@$VM_IP" "echo ok" >/dev/null 2>&1; then
        if ssh $SSH_OPTS "$VM_USER@$VM_IP" "docker info >/dev/null 2>&1" >/dev/null 2>&1; then
          echo "$NAME is ready at $VM_IP (user $VM_USER)." >&2
          return 0
        fi
      fi
    else
      virsh --connect qemu:///system start "$NAME" >/dev/null 2>&1 || true
    fi
    sleep 2
  done
  echo "$NAME did not become ready in time" >&2
  return 1
}

case "$cmd" in
  up)
    if ! virsh --connect qemu:///system dominfo "$NAME" >/dev/null 2>&1; then
      echo "VM $NAME not defined" >&2; exit 1
    fi
    if [[ $(virsh --connect qemu:///system dominfo "$NAME" 2>/dev/null | grep "State:") != *running* ]]; then
      virsh --connect qemu:///system start "$NAME" >/dev/null 2>&1 || true
      echo "starting $NAME..."
    fi
    wait_vm
    # forward WG port host->VM via socat
    if ! ss -ulpn 2>/dev/null | grep -q ":51820"; then
      nohup socat UDP-LISTEN:51820,reuseaddr,fork UDP:$VM_IP:51820 >/tmp/socat-51820.log 2>&1 &
      echo $! > /tmp/socat-51820.pid 2>/dev/null || true
      echo "forwarded host 51820/udp -> $VM_IP:51820 via socat (pid $!)" >&2
    fi
    ;;
  down)
    virsh --connect qemu:///system shutdown "$NAME" 2>/dev/null || true
    echo "sent shutdown to $NAME"
    ;;
  rm)
    virsh --connect qemu:///system destroy "$NAME" 2>/dev/null || true
    echo "destroyed $NAME (use 'virsh undefine' to delete disk)"
    ;;
  purge)
    virsh --connect qemu:///system destroy "$NAME" 2>/dev/null || true
    echo "purged $NAME (disk kept at /var/lib/libvirt/images/$NAME.qcow2)"
    ;;
  exec)
    shift
    # try ssh first, fall back to virsh console
    if ssh $SSH_OPTS "$VM_USER@$VM_IP" "true" 2>/dev/null; then
      ssh -t $SSH_OPTS "$VM_USER@$VM_IP" "${@:-bash}"
    else
      echo "VM not reachable via ssh, trying virsh console" >&2
      virsh --connect qemu:///system console "$NAME"
    fi
    ;;
  status)
    virsh --connect qemu:///system dominfo "$NAME" 2>&1 | head -20
    echo "---"
    virsh --connect qemu:///system domifaddr "$NAME" 2>&1 | head -20
    echo "--- docker ---"
    ssh $SSH_OPTS "$VM_USER@$VM_IP" "docker ps --format '{{.Names}} {{.Status}}' 2>&1 | head -20" 2>&1 | head -30 || echo "VM not reachable"
    ;;
  *)
    echo "usage: $0 {up|down|rm|purge|exec [cmd]|status}" >&2
    exit 1
    ;;
esac
