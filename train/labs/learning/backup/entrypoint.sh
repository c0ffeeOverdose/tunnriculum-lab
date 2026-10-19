#!/bin/bash
set -e

# debian already has a 'backup' user (no home) — give it one
mkdir -p /home/backup
usermod -d /home/backup -s /bin/bash backup
chown backup:backup /home/backup
echo "backup:${BACKUP_PASS}" | chpasswd

echo "${FLAG3:-flag_missing_flag3}" > /home/backup/flag3.txt
chown backup:backup /home/backup/flag3.txt

# "evidence": nightly dump of franchise sales + member data
cat > /home/backup/sales_backup_2026.sql <<'SQL'
-- Tang Wang Jeng HQ nightly backup (sales + members)
CREATE TABLE franchise_sales (
  branch VARCHAR(64), month VARCHAR(7), cups INT, revenue DECIMAL(10,2)
);
INSERT INTO franchise_sales VALUES
  ('Siam Square 01', '2026-06', 8412, 589340.00),
  ('Ari',            '2026-06', 6107, 427490.00),
  ('Chiang Mai Nim', '2026-06', 5520, 386400.00),
  ('Phuket Patong',  '2026-06', 4933, 345310.00);
CREATE TABLE members (
  id INT, username VARCHAR(64), phone VARCHAR(16), points INT
);
INSERT INTO members VALUES
  (1, 'mild_tea',   '081-111-2233', 420),
  (2, 'boba_fan99', '089-555-7788', 1337),
  (3, 'nong_mochi', '086-222-4455', 88);
SQL
chown backup:backup /home/backup/sales_backup_2026.sql

sed -i 's/^#\?PasswordAuthentication.*/PasswordAuthentication yes/' /etc/ssh/sshd_config
sed -i 's/^#\?PermitRootLogin.*/PermitRootLogin no/' /etc/ssh/sshd_config
echo 'AllowUsers backup' >> /etc/ssh/sshd_config

cat > /etc/motd <<'MOTD'
Tang Wang Jeng HQ — backup server
Authorized access only. All sessions are logged.
MOTD

exec /usr/sbin/sshd -D -e
