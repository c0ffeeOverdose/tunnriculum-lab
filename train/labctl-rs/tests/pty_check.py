#!/usr/bin/env python3
"""End-to-end flow tests for the labctl TUI, run INSIDE the VM.

Uses only the stdlib (unittest + pty) so the appliance VM needs no extra
packages. Run as the restricted player user to cover the real code path:

    sudo -u tunnriculum python3 train/labctl-rs/tests/pty_check.py

Each test drives the real binary through a pty and asserts on the rendered
screen. Full-screen redraws are forced by resizing the terminal (ratatui
renders incrementally otherwise, which makes diff-based checks unreliable).
NOTE: this file uses no `{}`-style formatting inside bash defaults.
"""
import fcntl
import os
import pty
import re
import select
import struct
import subprocess
import sys
import termios
import time
import unittest

TERM_H, TERM_W = 40, 120
ESC = "\x1b"
UP, DOWN, LEFT, RIGHT = f"{ESC}[A", f"{ESC}[B", f"{ESC}[D", f"{ESC}[C"
PGUP, PGDN = f"{ESC}[5~", f"{ESC}[6~"
HOME, END = f"{ESC}[H", f"{ESC}[F"
TAB, ENTER = "\t", "\r"
ANSI_RE = re.compile(r"\x1b\[[0-9;?]*[a-zA-Z]")


class Labctl:
    """One labctl process in a pty."""

    def __init__(self, rows=TERM_H, cols=TERM_W):
        self.rows, self.cols = rows, cols
        pid, fd = pty.fork()
        if pid == 0:
            os.environ["TERM"] = "xterm-256color"
            os.execvp("labctl", ["labctl"])
        self.pid, self.fd = pid, fd
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
        self.buf = b""
        self.drain(1.5)

    def drain(self, t=0.5):
        end = time.time() + t
        while time.time() < end:
            r, _, _ = select.select([self.fd], [], [], 0.05)
            if r:
                try:
                    self.buf += os.read(self.fd, 65536)
                except OSError:
                    return

    def send(self, keys, wait=0.4):
        # whole string in ONE write: split writes would break escape
        # sequences (ESC alone = "go back", then a stray "[C")
        os.write(self.fd, keys.encode())
        self.drain(wait)

    def snapshot(self, rows=None):
        """Force a full redraw by toggling the terminal height, return the
        raw frame text of the final (restored) size."""
        rows = rows or self.rows
        alt = rows + 1 if rows + 1 <= 60 else rows - 1
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack("HHHH", alt, self.cols, 0, 0))
        self.drain(0.7)
        self.buf = b""
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, self.cols, 0, 0))
        self.drain(0.7)
        return ANSI_RE.sub("", self.buf.decode(errors="replace"))

    def close(self):
        try:
            os.kill(self.pid, 9)
        except ProcessLookupError:
            pass

    # helpers -------------------------------------------------------------
    @staticmethod
    def flat(text):
        return text.replace(" ", "").replace("\r", "").replace("\n", "")

    def pct(self, text=None):
        """Output scroll percentage from the title, e.g. 'Output 95%'."""
        m = re.findall(r"Output (\d+)%", text if text is not None else self.snapshot())
        return int(m[-1]) if m else None


def goto_connect(lab):
    """Main -> Play (hotkey 2) -> Connect (3rd item)."""
    lab.send("2", 1.0)
    lab.send("j", 0.4)
    lab.send("j", 0.4)
    lab.send(ENTER, 1.0)


def goto_guide_step(lab, step):
    """Open the guide and flip to 1-based `step`."""
    lab.send("2", 1.0)
    lab.send(ENTER, 1.0)
    for _ in range(step - 1):
        lab.send(RIGHT, 0.6)


class Navigation(unittest.TestCase):
    def test_connect_menu_has_kali_manager(self):
        lab = Labctl()
        try:
            goto_connect(lab)
            s = Labctl.flat(lab.snapshot())
            self.assertIn("SSHintoKali", s)
            self.assertIn("RestartKali", s)
            self.assertIn("ResetKali(freshdesktop)", s)
        finally:
            lab.close()

    def test_guide_has_nine_steps(self):
        lab = Labctl()
        try:
            goto_guide_step(lab, 9)
            s = Labctl.flat(lab.snapshot())
            self.assertIn("step9/9", s)
        finally:
            lab.close()


class GuideChoiceBody(unittest.TestCase):
    """Regression: choosing a method must not hide the step intro (payload)."""

    def test_step3_choice_keeps_payload(self):
        lab = Labctl()
        try:
            goto_guide_step(lab, 3)
            lab.send(ENTER, 1.0)  # choose A. Burp
            s = Labctl.flat(lab.snapshot())
            self.assertIn("system($_GET", s, "payload must stay visible after choosing")
            self.assertIn("OpenBrowser", s, "chosen method steps must follow")
            # switch to B. curl
            lab.send("j", 0.5)
            lab.send(ENTER, 1.0)
            s = Labctl.flat(lab.snapshot())
            self.assertIn("type=image/png", s)
            self.assertIn("system($_GET", s)
        finally:
            lab.close()


class ConfirmDialog(unittest.TestCase):
    def test_default_is_safe_and_esc_cancels(self):
        lab = Labctl()
        try:
            goto_connect(lab)
            for _ in range(4):
                lab.send("j", 0.4)
            lab.send(ENTER, 1.0)  # open Reset Kali dialog
            s = Labctl.flat(lab.snapshot())
            self.assertIn("ResetKali?", s)
            self.assertIn("Keep", s)
            self.assertIn("Reset", s)
            # Esc = safe option
            lab.send(f"{ESC}", 1.0)
            s = Labctl.flat(lab.snapshot())
            self.assertIn("resetcancelled", s.lower())
            # reopen: Enter on default (Keep) must also cancel
            lab.send(ENTER, 1.0)
            lab.send(ENTER, 1.0)
            s = Labctl.flat(lab.snapshot())
            self.assertIn("resetcancelled", s.lower())
        finally:
            lab.close()


class ScrollDuringJob(unittest.TestCase):
    """Regression: output must scroll while a job streams (and resizing
    mid-stream must not panic the renderer)."""

    @staticmethod
    def start_logs(lab):
        lab.send("3", 1.0)          # Settings (PIN prompt opens automatically)
        lab.send("1337", 0.5)       # type PIN
        lab.send(ENTER, 1.0)        # unlock
        lab.send("j", 0.4)          # Logs
        lab.send(ENTER, 0.1)        # start (fast job: ~600 lines)
        lab.send(TAB, 0.1)          # focus output

    def test_detach_and_reattach_tail(self):
        lab = Labctl()
        try:
            self.start_logs(lab)
            self.drain_wait(lab, 4.0)
            tail = lab.pct()
            self.assertIsNotNone(tail, "scroll % must appear for long output")
            self.assertGreaterEqual(tail, 95)
            lab.send(PGUP, 0.8)
            up = lab.pct()
            self.assertLess(up, tail, "PageUp must detach from the tail")
            lab.send(END, 0.8)
            back = lab.pct()
            self.assertGreaterEqual(back, 95, "End must re-attach to the tail")
            # resize mid-stream is also the renderer panic regression test
            self.assertNotIn("panicked", lab.snapshot().lower())
        finally:
            lab.close()

    @staticmethod
    def drain_wait(lab, t):
        lab.drain(t)


class FlagSubmission(unittest.TestCase):
    """Regression: flag check must work for the restricted player user
    (read_flags falls back to `sudo -u lab lab.sh flags`)."""

    def test_flag1_accepted(self):
        # read the real flag the same way the app does
        try:
            out = subprocess.run(
                ["sudo", "-u", "lab", "/home/lab/tunnriculum/train/scripts/lab.sh", "flags"],
                capture_output=True, text=True, timeout=15)
            flag1 = dict(
                l.split("=", 1) for l in out.stdout.splitlines() if "=" in l
            ).get("FLAG1", "")
        except Exception:
            flag1 = ""
        self.assertTrue(flag1.startswith("flag{"), "could not read FLAG1")

        lab = Labctl()
        try:
            lab.send("2", 1.0)      # Play
            lab.send("j", 0.4)
            lab.send(ENTER, 1.0)    # Submit flags
            lab.send(ENTER, 0.8)    # flag1 input
            for ch in flag1:
                lab.send(ch, 0.01)
            lab.send(ENTER, 1.2)
            s = Labctl.flat(lab.snapshot())
            self.assertIn("flag1correct", s)
            self.assertNotIn("notinitialized", s.lower())
        finally:
            lab.close()


if __name__ == "__main__":
    unittest.main(verbosity=2)
