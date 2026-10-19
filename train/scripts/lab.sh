#!/usr/bin/env bash
# Manage the single learning-lab Compose project inside tunnriculum-lab VM.
# One VM runs exactly one instance; the project name and all IPs are fixed.
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
INSTANCE=tunnriculum
env_file="$root/instances/$INSTANCE.env"

action=${1:-}
endpoint=${2:-}

usage() {
  echo "usage: $0 {init|up|stop|down|reset|status|logs|smoke|peer|kali|kali-key|kali-restart|kali-reset|flags} [endpoint]" >&2
  exit 1
}

# Run directly on local Docker unless the tunnriculum-lab VM is present (or TUNN_VM_IP is set)
IN_VM=1
if [[ "${LOCAL:-0}" != "1" ]]; then
  if [[ -n "${TUNN_VM_IP:-}" ]] || (command -v virsh &>/dev/null && virsh --connect qemu:///system dominfo tunnriculum-lab &>/dev/null); then
    IN_VM=0
  fi
fi

# VM connection (host only) — dynamic address via qemu-guest-agent, .30 fallback, TUNN_VM_IP override
VM_USER=lab
VM_ROOT="/home/lab/tunnriculum"
VM_KEY="$HOME/.ssh/tunnriculum-lab"
SSH_OPTS="-i $VM_KEY -o StrictHostKeyChecking=no -o ConnectTimeout=5"
vm_ip() {
  if [[ -n ${TUNN_VM_IP:-} ]]; then
    echo "$TUNN_VM_IP"; return
  fi
  local mac out
  mac=$(virsh --connect qemu:///system dumpxml tunnriculum-lab 2>/dev/null | grep -oE "52:54:00:[0-9a-f:]{8}|0a:00:27:[0-9a-f:]{8}" | head -1)
  out=$(virsh --connect qemu:///system domifaddr tunnriculum-lab --source agent 2>/dev/null \
    | grep "$mac" | grep -oE '([0-9]{1,3}\.){3}[0-9]{1,3}' | head -1)
  echo "${out:-192.168.122.30}"
}
VM_SSH="$VM_USER@$(vm_ip)"

# --- Host delegation: sync code and run inside VM ---
if (( IN_VM == 0 )); then
  # ensure VM is up for any action except init that may not need it
  if [[ "$action" != "init" ]]; then
    "$root/scripts/vm-up.sh" up >/dev/null 2>&1 || true
  fi

  sync_code() {
    rsync -az --delete --exclude '.git' --exclude 'instances/*.env' --exclude 'dist' --exclude '*.qcow2' \
      -e "ssh -i $VM_KEY -o StrictHostKeyChecking=no" "$root/" "$VM_SSH:$VM_ROOT/train/" >/dev/null 2>&1 || true
  }

  case "$action" in
    init)
      if ssh $SSH_OPTS "$VM_SSH" "test -f $VM_ROOT/train/instances/$INSTANCE.env" 2>/dev/null; then
        echo "$VM_ROOT/train/instances/$INSTANCE.env already exists on VM; use 'reset' to wipe" >&2; exit 1
      fi
      sync_code
      ssh $SSH_OPTS "$VM_SSH" "cd $VM_ROOT/train && ./scripts/lab.sh init" 2>&1
      # copy env back to host for reference (mode 0600)
      mkdir -p "$root/instances"
      scp -i "$VM_KEY" -o StrictHostKeyChecking=no "$VM_SSH:$VM_ROOT/train/instances/$INSTANCE.env" "$env_file" 2>/dev/null || true
      chmod 600 "$env_file" 2>/dev/null || true
      echo "Synced env to $env_file (host copy)"
      exit 0
      ;;
    up)
      sync_code
      ssh $SSH_OPTS "$VM_SSH" "cd $VM_ROOT/train && ./scripts/lab.sh up" 2>&1
      exit $?
      ;;
    stop|down|reset|status|logs|smoke|peer|kali|kali-key|kali-restart|kali-reset|flags)
      # peer needs endpoint
      if [[ "$action" == "peer" ]]; then
        ssh $SSH_OPTS "$VM_SSH" "cd $VM_ROOT/train && ./scripts/lab.sh peer $endpoint" 2>&1
      else
        ssh $SSH_OPTS "$VM_SSH" "cd $VM_ROOT/train && ./scripts/lab.sh $action" 2>&1
      fi
      exit $?
      ;;
    *) usage ;;
  esac
fi

# --- Direct execution (inside VM or local Docker host): ---

env_value() {
  local wanted=$1 key value
  while IFS='=' read -r key value; do
    [[ $key == "$wanted" ]] && { printf '%s' "$value"; return 0; }
  done < "$env_file"
  return 1
}

compose() {
  if docker compose version &>/dev/null; then
    docker compose -p "$INSTANCE" --env-file "$env_file" \
      -f "$root/labs/learning/compose.yml" "$@"
  else
    docker-compose -p "$INSTANCE" --env-file "$env_file" \
      -f "$root/labs/learning/compose.yml" "$@"
  fi
}

init() {
  [[ -e $env_file ]] && { echo "$env_file already exists; use 'reset' to wipe the lab" >&2; exit 1; }

  random_token() {
    openssl rand -base64 24 | tr '+/' '-_' | tr -d '=\n'
  }

  db_pass=$(random_token)
  backup_pass=$(random_token)
  # vncpasswd only uses the first 8 chars
  kali_vnc_pass=$(random_token | head -c 8)
  # keypair so labctl can SSH into the Kali box without a password
  kali_key="$root/instances/${INSTANCE}_kali_key"
  ssh-keygen -t ed25519 -N '' -C tunnriculum-labctl -f "$kali_key" -q
  kali_pubkey=$(cat "${kali_key}.pub")

  umask 077
  mkdir -p "$root/instances"
  cat > "$env_file" <<EOF
PREFIX=${INSTANCE}-
WG_PORT=51820
WG_SERVERURL=auto

NET_DMZ=10.100.100.0/24
NET_INT=10.100.200.0/24
NET_RES=10.100.250.0/24

IP_WG=10.100.100.2
IP_KALI=10.100.100.50
IP_DB=10.100.200.20
IP_WEB_DMZ=10.100.100.10
IP_WEB_INT=10.100.200.5
IP_POS_INT=10.100.200.10
IP_POS_RES=10.100.250.5
IP_BACKUP=10.100.250.10

DB_PASS=$db_pass
POS_PASS=$db_pass
BACKUP_PASS=$backup_pass
KALI_VNC_PASS=$kali_vnc_pass
KALI_SSH_PUBKEY=$kali_pubkey
KALI_PORT=6080
KALI_RDP_PORT=3389
KALI_SSH_PORT=2222
OPTIONS_PIN=1337

FLAG1=flag{$(random_token)}
FLAG2=flag{$(random_token)}
FLAG3=flag{$(random_token)}
EOF
  chmod 600 "$env_file"

  echo "Created $env_file with private flags and passwords (mode 0600, not in Git)."
  echo "Run: $0 up"
}

up() {
  compose up -d --build --force-recreate
}

stop() {
  compose stop
}

down() {
  compose down --volumes --remove-orphans
}

reset() {
  down
  up
}

status() {
  # -a: show stuck Created containers too, not just running ones
  compose ps -a
}

logs() {
  compose logs --tail=100
}

smoke() {
  [[ -f $env_file ]] || { echo "missing $env_file; run '$0 init'" >&2; exit 1; }

  web="$INSTANCE-web"
  wg="$INSTANCE-wg"
  pos="$INSTANCE-intranet"
  backup="$INSTANCE-backup"
  kali_c="$INSTANCE-kali"

  exec_docker() {
    docker exec "$1" "${@:2}"
  }

  inspect_running() {
    docker inspect -f '{{.State.Running}}' "$1" 2>/dev/null || echo "false"
  }

  for container in "$web" "$wg" "$pos" "$backup" "$kali_c"; do
    [[ $(inspect_running "$container") == "true" ]] || { echo "container $container is not running" >&2; exit 1; }
  done

  web_dmz=$(env_value IP_WEB_DMZ)
  pos_internal=$(env_value IP_POS_INT)
  backup_ip=$(env_value IP_BACKUP)

  exec_docker "$wg" curl -fsS "http://$web_dmz/" >/dev/null
  ! exec_docker "$wg" curl -fsS --max-time 3 "http://$pos_internal/login.php" >/dev/null 2>&1
  exec_docker "$web" curl -fsS "http://$pos_internal/login.php" >/dev/null
  ! exec_docker "$web" nc -zvw3 "$backup_ip" 22 >/dev/null 2>&1
  exec_docker "$pos" nc -zvw3 "$backup_ip" 22 >/dev/null 2>&1
  # kali desktop: DMZ-only view, same as a WireGuard player
  exec_docker "$kali_c" curl -fsS --max-time 5 "http://$web_dmz/" >/dev/null
  ! exec_docker "$kali_c" curl -fsS --max-time 3 "http://$pos_internal/login.php" >/dev/null 2>&1

  echo "Topology smoke test passed."
}

peer() {
  # default endpoint: this machine's primary address (appliance mode)
  if [[ -z "$endpoint" || "$endpoint" == "127.0.0.1" ]]; then
    endpoint=$(ip -4 -o addr show scope global | awk '{print $4}' | cut -d/ -f1 | head -1)
  fi
  [[ $endpoint =~ ^[A-Za-z0-9.-]+$ ]] || { echo "public host must be a hostname or IPv4 address" >&2; exit 1; }
  wg_port=$(env_value WG_PORT)
  web_ip=$(env_value IP_WEB_DMZ)
  docker exec "$INSTANCE-wg" cat /config/peer1/peer1.conf \
    | sed -E '/^(DNS|ListenPort) =/d; s|^Endpoint = .*|Endpoint = '"$endpoint:$wg_port"'|' \
    | { cat; printf '\nPersistentKeepalive = 25\n'; }
  # usage as comments — wg-quick ignores them, players keep the how-to with the config
  cat <<EOF
# ── How to connect ─────────────────────────────────────
# 1. Save this whole output to a file:   lab.sh peer $endpoint > peer1.conf
#    (from the TUI: WireGuard config, then copy the output)
# 2. Bring the tunnel up:                sudo wg-quick up peer1.conf
# 3. Open the shop:                      http://$web_ip/
# Only the DMZ is reachable — everything deeper needs a pivot.
# Done?                                  sudo wg-quick down peer1.conf
# ────────────────────────────────────────────────────────
EOF
}

kali() {
  [[ -f $env_file ]] || { echo "missing $env_file; run '$0 init'" >&2; exit 1; }
  local port rdp_port ssh_port pass ip
  port=$(env_value KALI_PORT || true)
  rdp_port=$(env_value KALI_RDP_PORT || true)
  ssh_port=$(env_value KALI_SSH_PORT || true)
  pass=$(env_value KALI_VNC_PASS || true)
  ip=$(ip -4 -o addr show scope global | awk '{print $4}' | cut -d/ -f1 | head -1)
  cat <<EOF
Kali desktop : http://${ip:-<vm-ip>}:${port:-6080}/vnc.html
RDP          : xfreerdp /v:${ip:-<vm-ip>}:${rdp_port:-3389} /u:kali /p:<password> /cert:ignore /size:1920x1080 +clipboard
SSH          : ssh -p ${ssh_port:-2222} kali@${ip:-<vm-ip>}
Password     : ${pass:-<not set>}   (same for noVNC / RDP / SSH, user kali)

noVNC: open the URL, Connect, type the password.
RDP/SSH: native clients, same desktop/tools, clipboard + resize.
No WireGuard needed — the desktop sits in the DMZ, pivot from there.
EOF
}

# restart the kali container only (keeps the container layer: home files stay)
kali_restart() {
  [[ -f $env_file ]] || { echo "missing $env_file; run '$0 init'" >&2; exit 1; }
  echo "Restarting kali (keeps home files)..."
  compose restart kali
  echo "kali restarted. URL: http://<vm-ip>:$(env_value KALI_PORT || echo 6080)/vnc.html"
}

# recreate the kali container from scratch (fresh home; other services & flags untouched)
kali_reset() {
  [[ -f $env_file ]] || { echo "missing $env_file; run '$0 init'" >&2; exit 1; }
  echo "Recreating kali with a fresh desktop (other services untouched)..."
  compose up -d --force-recreate kali
  echo "kali reset done. URL: http://<vm-ip>:$(env_value KALI_PORT || echo 6080)/vnc.html"
}

# print the three flag values (labctl flag submission reads these)
flags() {
  [[ -f $env_file ]] || { echo "missing $env_file; run '$0 init'" >&2; exit 1; }
  for k in FLAG1 FLAG2 FLAG3; do
    printf '%s=%s\n' "$k" "$(env_value "$k" || true)"
  done
}

# print the instance SSH key (private) — used by labctl for passwordless SSH
kali_key() {
  local key="$root/instances/${INSTANCE}_kali_key"
  [[ -f $key ]] || { echo "missing $key; run '$0 init'" >&2; exit 1; }
  cat "$key"
}

case "$action" in
  init) init ;;
  up) up ;;
  stop) stop ;;
  down) down ;;
  reset) reset ;;
  status) status ;;
  logs) logs ;;
  smoke) smoke ;;
  peer) peer ;;
  kali) kali ;;
  kali-key) kali_key ;;
  flags) flags ;;
  kali-restart) kali_restart ;;
  kali-reset) kali_reset ;;
  *) usage ;;
esac
