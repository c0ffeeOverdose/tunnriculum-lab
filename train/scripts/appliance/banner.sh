#!/usr/bin/env bash
# Print the appliance banner: where the lab lives and how to reach it.
set -u

ip=$(ip -4 -o addr show scope global | awk '{print $4}' | cut -d/ -f1 | head -1)

cat <<EOF

  ============================================
   Tunnriculum — double-pivot network lab
  ============================================
   Lab address : ${ip:-unknown}
   WireGuard   : ${ip:-unknown}:51820 (udp)

   Play now (browser) : http://${ip:-<ip>}:6080/vnc.html
                        (Kali desktop, password from the TUI)
   Play own machine   : WireGuard or SSH — see the TUI Play menu
   Tunnriculum UI     : ssh tunnriculum@${ip:-<ip>}
   TUI PIN            : 1337

   The lab stack starts automatically. Use the
   Tunnriculum TUI Play menu for connection info
   and flag submission.
  ============================================

EOF
