"""Exercise the Linux controller without changing the user's wallpaper."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]


class DesktopLauncherTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="ascii desktop ")
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.bin = self.directory / "bin"
        self.bin.mkdir()
        self.env = dict(os.environ, PATH=f"{self.bin}:{os.environ['PATH']}")
        self.env["DESKTOP_TEST_DIR"] = str(self.directory)
        self.env["XDG_DATA_HOME"] = str(self.directory / 'data $cash "quoted" %percent')
        self.env["XDG_CURRENT_DESKTOP"] = "test-desktop"
        for name in ("desktop.sh", "install-desktop.sh"):
            shutil.copy(ROOT / "shells/linux" / name, self.directory / name)
        (self.directory / "icon.png").write_bytes(b"test icon")
        shutil.copytree(ROOT / "shells/linux/gnome", self.directory / "gnome")
        self.write_executable(self.bin / "zenity", '''#!/usr/bin/env python3
import json, os, pathlib, sys
root = pathlib.Path(os.environ["DESKTOP_TEST_DIR"])
queue = root / "dialogs.json"
responses = json.loads(queue.read_text())
response = responses.pop(0)
queue.write_text(json.dumps(responses))
if response is None:
    sys.exit(1)
print(response)
''')

    def write_executable(self, path, content):
        path.write_text(content)
        path.chmod(0o755)

    def run_controller(self, responses, fail=False):
        (self.directory / "dialogs.json").write_text(json.dumps(responses))
        self.write_executable(self.directory / "aa-linux", '''#!/usr/bin/env python3
import json, os, pathlib, signal, sys, time
root = pathlib.Path(os.environ["DESKTOP_TEST_DIR"])
if sys.argv[1:] == ["--autostart-status"]:
    print("false")
    sys.exit(0)
(root / "arguments.json").write_text(json.dumps(sys.argv[1:]))
def stop(*_):
    (root / "stopped").touch()
    sys.exit(0)
signal.signal(signal.SIGTERM, stop)
''' + ("sys.exit(1)\n" if fail else "while True: time.sleep(0.05)\n"))
        subprocess.run(["bash", str(self.directory / "desktop.sh")],
                       env=self.env, check=True, timeout=15)
        self.assertEqual(json.loads((self.directory / "dialogs.json").read_text()), [])

    def test_selection_and_close_stop_only_owned_process(self):
        unrelated = subprocess.Popen(["sleep", "20"])
        self.addCleanup(unrelated.wait)
        self.addCleanup(unrelated.terminate)
        self.run_controller(["Choose scene", "matrix", "Choose theme", "amber",
                             "Start wallpaper", None])
        self.assertEqual(json.loads((self.directory / "arguments.json").read_text()),
                         ["matrix", "amber"])
        self.assertTrue((self.directory / "stopped").exists())
        self.assertIsNone(unrelated.poll())

    def test_cancel_preserves_selection(self):
        self.run_controller(["Choose scene", "pipes", "Choose scene", None,
                             "Choose theme", "ice", "Choose theme", None,
                             "Start wallpaper", "Quit"])
        self.assertEqual(json.loads((self.directory / "arguments.json").read_text()),
                         ["pipes", "ice"])

    def test_explicit_stop(self):
        self.run_controller(["Start wallpaper", "Stop wallpaper", "Quit"])
        self.assertTrue((self.directory / "stopped").exists())

    def test_failed_host_shows_error_dialog(self):
        self.run_controller(["Start wallpaper", "", "Quit"], fail=True)

    def test_install_with_reserved_characters_in_path(self):
        self.write_executable(self.directory / "aa-linux", "#!/bin/sh\nexit 0\n")
        self.write_executable(self.directory / "aa", "#!/bin/sh\nexit 0\n")
        subprocess.run(["bash", str(self.directory / "install-desktop.sh")],
                       env=self.env, check=True, timeout=10)
        data = Path(self.env["XDG_DATA_HOME"])
        entry = data / "applications/ascii-arcade.desktop"
        subprocess.run(["desktop-file-validate", str(entry)], check=True)
        self.assertTrue(os.access(data / "ascii-arcade/desktop.sh", os.X_OK))
        self.assertIn("%%percent", entry.read_text())
        extension = data / "gnome-shell/extensions/ascii-arcade@builder106"
        self.assertTrue((extension / "extension.js").is_file())
        self.assertTrue((extension / "schemas/gschemas.compiled").is_file())

    def test_gnome_controller_uses_extension_settings_and_stops_on_close(self):
        self.env["XDG_CURRENT_DESKTOP"] = "ubuntu:GNOME"
        self.env["WAYLAND_DISPLAY"] = "wayland-test"
        schema = Path(self.env["XDG_DATA_HOME"]) / "gnome-shell/extensions/ascii-arcade@builder106/schemas"
        schema.mkdir(parents=True)
        (schema / "gschemas.compiled").touch()
        self.write_executable(self.bin / "gnome-extensions", "#!/bin/sh\nprintf '%s\\n' ascii-arcade@builder106\n")
        self.write_executable(self.bin / "gsettings", '''#!/usr/bin/env python3
import os, pathlib, sys
defaults = {
    "scene": "'donut'",
    "theme": "'hacker'",
    "doom-enabled": "false",
    "opaque-background": "true",
    "idle-cycle": "false",
    "capture-keys-for-doom": "true",
    "launch-at-login": "false",
}
if sys.argv[-3] == "get":
    print(defaults[sys.argv[-1]])
    sys.exit(0)
with (pathlib.Path(os.environ["DESKTOP_TEST_DIR"]) / "settings.log").open("a") as log:
    log.write(" ".join(sys.argv[2:]) + "\\n")
''')
        self.run_controller(["Start wallpaper", "Quit"])
        calls = (self.directory / "settings.log").read_text()
        self.assertIn("scene donut", calls)
        self.assertIn("theme hacker", calls)
        self.assertIn("running true", calls)
        self.assertTrue(calls.rstrip().endswith("running false"))
        self.assertFalse((self.directory / "arguments.json").exists())


if __name__ == "__main__":
    unittest.main()
