# Boots the FreeBSD cloud image under a private QEMU, gets a root shell on the serial port from a nuageinit script on a `cidata` disc (the
# image has no serial login), sends the binary as base64, and runs the same shape checks as the Linux smokes (`shape.py`) with the
# commands executed inside the guest. Usage: freebsd-serial.py <binary> <labels file> <image>
#
# The output of each command is carried back as base64 from files in the guest, so the bytes are exact and the console's line-end
# translation never touches them. Colour runs go through `script`, which gives the program a pseudo-terminal. Only our own QEMU process
# is ever stopped, by its PID.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

import base64
import os
import re
import socket
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import shape  # noqa: E402

PROMPT_END = "@@END@@"


class Guest:
	def __init__(self, image, scratch):
		self.path = os.path.join(scratch, "serial.sock")
		seed = os.path.join(scratch, "seed")
		os.mkdir(seed)
		with open(os.path.join(seed, "meta-data"), "w", encoding="ascii") as handle:
			handle.write("instance-id: afterfetch\nlocal-hostname: freebsd\n")
		# nuageinit runs the `runcmd` entries once, at boot. They turn the serial port's login into an automatic root login (the image has
		# no password to type) and restart that getty, and say so on the console.
		with open(os.path.join(seed, "user-data"), "w", encoding="ascii") as handle:
			handle.write(
				"#cloud-config\nruncmd:\n"
				"  - echo 'autoroot:al=root:tc=3wire:' >> /etc/gettytab\n"
				"  - sed -i '' 's|^ttyu0.*|ttyu0 \"/usr/libexec/getty autoroot\" vt100 on secure|' /etc/ttys\n"
				"  - echo AFREADY > /dev/console\n"
				"  - kill -HUP 1\n"
			)
		disc = os.path.join(scratch, "seed.iso")
		subprocess.run(["genisoimage", "-quiet", "-V", "cidata", "-J", "-r", "-o", disc, seed], check=True)
		self.qemu = subprocess.Popen(
			["qemu-system-x86_64", "-enable-kvm", "-cpu", "host", "-m", "1024", "-smp", "2", "-display", "none", "-monitor", "none",
			 "-serial", f"unix:{self.path},server=on,wait=off", "-nic", "none", "-cdrom", disc,
			 "-drive", f"file={image},if=virtio,format=qcow2,snapshot=on"],
			stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
		self.log = b""
		for _ in range(50):
			if os.path.exists(self.path):
				break
			time.sleep(0.1)
		self.sock = socket.socket(socket.AF_UNIX)
		self.sock.connect(self.path)
		self.sock.settimeout(1.0)

	def close(self):
		self.qemu.terminate()
		try:
			self.qemu.wait(timeout=15)
		except subprocess.TimeoutExpired:
			self.qemu.kill()

	def read_until(self, pattern, timeout):
		deadline = time.time() + timeout
		found = None
		while time.time() < deadline:
			found = re.search(pattern, self.log)
			if found:
				break
			try:
				chunk = self.sock.recv(4096)
			except socket.timeout:
				continue
			self.log += chunk
		if not found:
			raise shape.Failure(f"timed out waiting for {pattern!r}; the console ended with {self.log[-300:]!r}")
		consumed = self.log[: found.end()]
		self.log = self.log[found.end():]
		return consumed, found

	def send(self, text):
		# The guest's UART has a small receive FIFO and no flow control, so a long line is sent in short, paced pieces.
		data = text.encode("ascii")
		for at in range(0, len(data), 16):
			self.sock.sendall(data[at:at + 16])
			time.sleep(0.002)

	def boot(self):
		self.read_until(rb"AFREADY", 300)
		self.read_until(rb"root@freebsd:~ # ", 90)
		self.send("exec /bin/sh\n")
		time.sleep(2)
		# The shell's own line editor redraws long lines with backspaces; plain terminal input is what the harness needs.
		self.send("set +o emacs; set +o vi; PS1=''; stty -echo; export TERM=xterm\n")
		time.sleep(1)
		self.log = b""

	def shell(self, command, timeout=60):
		"""Runs `command` in the guest's shell and returns (output, status)."""
		self.send(f"{command}\necho {PROMPT_END}$?\n")
		try:
			consumed, found = self.read_until(PROMPT_END.encode() + rb"(\d+)\r?\n", timeout)
		except shape.Failure as failure:
			raise shape.Failure(f"while running {command[:60]!r}: {failure}") from None
		return consumed[: found.start()].replace(b"\r\n", b"\n"), int(found.group(1))

	def put(self, local, remote):
		data = base64.b64encode(open(local, "rb").read()).decode("ascii")
		self.shell(f": > {remote}.b64")
		for at in range(0, len(data), 384):
			self.shell(f"printf '%s' '{data[at:at + 384]}' >> {remote}.b64")
		_, status = self.shell(f"base64 -d < {remote}.b64 > {remote} && chmod +x {remote}")
		if status != 0:
			raise shape.Failure("sending the binary failed")

	def fetch_bytes(self, remote):
		out, status = self.shell(f"base64 < {remote}")
		return base64.b64decode(b"".join(out.split())) if status == 0 else b""


def main(argv):
	binary, labels, image = argv[1:4]
	scratch = tempfile.mkdtemp(prefix="afterfetch-freebsd.")
	guest = Guest(image, scratch)
	try:
		print("booting FreeBSD (this takes up to a minute)")
		guest.boot()
		release, _ = guest.shell("freebsd-version -ku; uname -srm")
		print(release.decode("ascii", "replace").strip())
		guest.put(binary, "/tmp/afetch")
		remote = "/tmp/afetch"
		# Debugging aid: AF_EXEC="commands" runs them in the guest, prints the output and stops.
		if os.environ.get("AF_EXEC"):
			output, status = guest.shell(os.environ["AF_EXEC"])
			print(output.decode("utf-8", "replace"), f"[status {status}]")
			if os.environ.get("AF_FETCH"):
				with open(os.environ["AF_FETCH_TO"], "wb") as handle:
					handle.write(guest.fetch_bytes(os.environ["AF_FETCH"]))
			return 0

		def run_in_guest(command, extra_environment=None, **_):
			environment = dict(extra_environment or {})
			assignments = " ".join(f"{name}='{value}'" for name, value in environment.items())
			arguments = " ".join(f"'{part}'" for part in command[1:])
			_, status = guest.shell(f"{assignments} {remote} {arguments} >/tmp/out 2>/tmp/err")
			return subprocess.CompletedProcess(command, status, guest.fetch_bytes("/tmp/out"), guest.fetch_bytes("/tmp/err"))

		real_run = subprocess.run

		def run_with_closed_output(command, *args, **kwargs):
			# shape.py asks for output to a full device. The guest has no /dev/full, so a closed standard output stands in: the write fails.
			if kwargs.get("stdout") is not None and command[0] == remote:
				_, status = guest.shell(f"{remote} {' '.join(command[1:])} >&- 2>/dev/null")
				return subprocess.CompletedProcess(command, status)
			return real_run(command, *args, **kwargs)

		shape.run = run_in_guest
		shape.subprocess.run = run_with_closed_output
		shape.check_plain([remote], labels)
		# The first run's JSON and picture, for the record.
		print(run_in_guest([remote, "--json"]).stdout.decode("utf-8", "replace").strip())
		print(run_in_guest([remote, "--plain"]).stdout.decode("utf-8", "replace"))
		for depth, term, colorterm in (("16", "xterm", ""), ("256", "xterm-256color", ""), ("truecolour", "xterm-256color", "truecolor"), ("none", "dumb", "")):
			guest.shell(f"TERM={term} COLORTERM='{colorterm}' script -q /tmp/typescript {remote} >/dev/null 2>&1 </dev/null")
			capture = os.path.join(scratch, f"colour-{depth}.txt")
			with open(capture, "wb") as handle:
				handle.write(guest.fetch_bytes("/tmp/typescript"))
			shape.check_colour(capture, depth)
	except shape.Failure as failure:
		print(f"FAIL: {failure}", file=sys.stderr)
		return 1
	finally:
		guest.close()
		subprocess.run(["rm", "-r", scratch], check=False)
	return 0


if __name__ == "__main__":
	sys.exit(main(sys.argv))
