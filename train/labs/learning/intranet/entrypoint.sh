#!/bin/bash
set -e

# flag2 lives on the POS host itself
useradd -m -s /bin/bash posadmin 2>/dev/null || true
echo "posadmin:${POS_PASS}" | chpasswd
echo "${FLAG2:-flag_missing_flag2}" > /home/posadmin/flag2.txt
chown posadmin:posadmin /home/posadmin/flag2.txt

sed -i 's/^#\?PasswordAuthentication.*/PasswordAuthentication yes/' /etc/ssh/sshd_config
/usr/sbin/sshd

grep -q hq.tangwangjeng.corp /etc/hosts || echo "${HQ_HOST} hq.tangwangjeng.corp" >> /etc/hosts

exec apache2-foreground
