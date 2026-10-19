#!/bin/bash
set -e

# Story: the intern reused the DB password for the 'webadmin' user (found in config.php)
useradd -m -s /bin/bash webadmin 2>/dev/null || true
echo "webadmin:${DB_PASS}" | chpasswd
# Plant flag1 where only a shell on this box can read it — group www-data so RCE can read it
echo "${FLAG1:-flag_missing_flag1}" > /home/flag1.txt
chown webadmin:www-data /home/flag1.txt
chmod 640 /home/flag1.txt
# www-data shell for web-shell access
usermod -s /bin/bash www-data

sed -i 's/^#\?PasswordAuthentication.*/PasswordAuthentication yes/' /etc/ssh/sshd_config
/usr/sbin/sshd

# /etc/hosts entry so the intranet feels "internal"
grep -q intranet.tangwangjeng.corp /etc/hosts || echo "${INTRANET_HOST} intranet.tangwangjeng.corp" >> /etc/hosts

exec apache2-foreground
