# Kali attack desktop for players without WireGuard — browser-only access via noVNC.
# Attached to the DMZ network ONLY: same view as a WireGuard player, must pivot to go deeper.
# Layered RUNs on purpose: an unstable network only rebuilds the failed chunk.
FROM kalilinux/kali-rolling

ENV DEBIAN_FRONTEND=noninteractive \
    TERM=xterm

# 1. desktop (the big layer — cache it)
RUN apt-get update && \
    apt-get install -y --no-install-recommends kali-desktop-xfce dbus-x11 && \
    rm -rf /var/lib/apt/lists/*

# 2. attack tools the intended chain needs (keep this lean on purpose)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      firefox-esr nmap curl openssh-client openssh-server proxychains4 sshuttle sshpass \
      iproute2 iputils-ping netcat-openbsd burpsuite \
      xfce4-terminal && \
    rm -rf /var/lib/apt/lists/*

# 3. ligolo-ng proxy (pinned release; the agent goes on the pivot boxes)
ADD https://github.com/nicocha30/ligolo-ng/releases/download/v0.9.1/ligolo-ng_proxy_0.9.1_linux_amd64.tar.gz /tmp/ligolo.tgz
RUN tar xzf /tmp/ligolo.tgz -C /tmp && \
    install -m 755 /tmp/proxy /usr/local/bin/proxy && \
    rm -rf /tmp/ligolo.tgz /tmp/proxy /tmp/LICENSE /tmp/README.md

# 3. remote desktop plumbing: TigerVNC+noVNC (browser) and xrdp (native RDP client)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      tigervnc-standalone-server tigervnc-tools novnc websockify sudo xrdp && \
    rm -rf /var/lib/apt/lists/*

# normal desktop user, like a stock Kali box (sudo NOPASSWD so nmap -sS etc. work without prompts)
RUN useradd -m -s /bin/bash kali && \
    echo 'kali:kali' | chpasswd && \
    usermod -aG sudo kali && \
    printf 'kali ALL=(ALL) NOPASSWD:ALL\n' > /etc/sudoers.d/kali && \
    chmod 440 /etc/sudoers.d/kali

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

EXPOSE 6080
ENTRYPOINT ["/entrypoint.sh"]
