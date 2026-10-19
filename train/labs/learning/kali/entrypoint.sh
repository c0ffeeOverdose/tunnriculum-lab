#!/bin/bash
# Player-facing Kali desktop: TigerVNC :5901 (localhost only) -> noVNC :6080.
# Runs as the normal 'kali' user; root via sudo (NOPASSWD) when a tool needs it.
set -e

: "${KALI_VNC_PASS:?KALI_VNC_PASS required}"
# vncpasswd silently truncates to 8 chars — enforce it here so the shown password always works
PASS=$(printf '%s' "$KALI_VNC_PASS" | head -c 8)
[ ${#PASS} -ge 6 ] || { echo "KALI_VNC_PASS must be 6-8 characters" >&2; exit 1; }

# SSH door: same random password as VNC/RDP (one password, three doors)
mkdir -p /run/sshd
echo "kali:$PASS" | chpasswd
# labctl's key (passwordless SSH from the console)
mkdir -p /home/kali/.ssh
echo "${KALI_SSH_PUBKEY:-}" > /home/kali/.ssh/authorized_keys
chown -R kali:kali /home/kali/.ssh
chmod 700 /home/kali/.ssh
chmod 600 /home/kali/.ssh/authorized_keys
/usr/sbin/sshd

su - kali -c 'bash -s' <<EOF
set -e
VNCDIR="\$HOME/.config/tigervnc"
mkdir -p "\$VNCDIR"
printf '%s\n' "$PASS" | vncpasswd -f > "\$VNCDIR/passwd"
chmod 600 "\$VNCDIR/passwd"

cat > "\$VNCDIR/xstartup" <<'XEOF'
#!/bin/sh
unset SESSION_MANAGER
unset DBUS_SESSION_BUS_ADDRESS
exec dbus-launch --exit-with-session startxfce4
XEOF
chmod +x "\$VNCDIR/xstartup"

# clean stale locks from an unclean container stop
rm -f "\$VNCDIR"/*:1.pid "\$VNCDIR"/*.log /tmp/.X11-unix/X1 /tmp/.X1-lock

tigervncserver :1 -geometry 1280x800 -depth 24

websockify --web /usr/share/novnc --heartbeat 30 6080 localhost:5901 &
EOF

# xrdp: proxy mode into the SAME VNC session (:5901) — same VNC password from an RDP client
mkdir -p /var/run/xrdp /var/.xrdp
chown xrdp:xrdp /var/run/xrdp /var/.xrdp
# TLS cert: classic RDP security has a MAC bug with FreeRDP3 — must use TLS
[ -f /etc/xrdp/cert.pem ] || openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
  -keyout /etc/xrdp/key.pem -out /etc/xrdp/cert.pem \
  -subj "/CN=kali" >/dev/null 2>&1
chown xrdp:xrdp /etc/xrdp/cert.pem /etc/xrdp/key.pem
cat > /etc/xrdp/xrdp.ini <<'XRDP'
[Globals]
port=3389
security_layer=negotiate
crypt_level=medium
certificate=/etc/xrdp/cert.pem
key_file=/etc/xrdp/key.pem
bitmap_compression=true
max_bpp=32
LogLevel=INFO
LogFile=/dev/stdout
EnableSyslog=false
[Channels]
rdpdr=true
rdpsnd=true
cliprdr=true
[Xvnc]
name=Kali (VNC proxy)
lib=libvnc.so
ip=127.0.0.1
port=5901
username=ask
password=ask
XRDP

# wait for the VNC session (started inside su above) before proxying
for i in $(seq 1 20); do
  (exec 3<>/dev/tcp/127.0.0.1/5901) 2>/dev/null && { exec 3>&- 3<&-; break; }
  sleep 1
done

exec xrdp --nodaemon
