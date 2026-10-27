"""Check the extension in an isolated nested GNOME 46 Wayland session.

Requires GNOME Shell 46, GJS, Xvfb, ImageMagick, and D-Bus. Run on Linux.
"""

import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
UUID = "ascii-arcade@builder106"
SCHEMA = "org.gnome.shell.extensions.ascii-arcade"
PROBE = "ascii-arcade-test@local"

PROBE_SOURCE = '''
import GLib from 'gi://GLib';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
export default class Probe extends Extension {
    enable() {
        this._source = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 250, () => {
            const entry = Main.extensionManager.lookup('ascii-arcade@builder106');
            const app = entry?.stateObj;
            const report = {state: entry?.state, errors: entry?.errors};
            try {
                report.pid = app?._run?.process.get_identifier() ?? null;
                report.texture = app?._image?.get_texture()?.get_width() ?? 0;
                report.actors = app?._group?.get_n_children() ?? 0;
                report.monitors = Main.layoutManager.monitors.length;
                report.background = app?._group?.get_parent() === Main.layoutManager._backgroundGroup;
                Main.overview.hide();
            } catch (error) { report.error = error.stack; }
            GLib.file_set_contents(GLib.build_filenamev([GLib.get_user_runtime_dir(), 'arcade-probe.json']), JSON.stringify(report));
            return GLib.SOURCE_CONTINUE;
        });
    }
    disable() { GLib.source_remove(this._source); }
}
'''


def command(*args: str) -> str:
    return subprocess.check_output(args, text=True, stderr=subprocess.STDOUT, timeout=15).strip()


def await_report(runtime: Path, predicate, timeout: int = 30) -> dict:
    deadline = time.monotonic() + timeout
    latest = {}
    while time.monotonic() < deadline:
        try:
            latest = json.loads((runtime / "arcade-probe.json").read_text())
            if latest.get("error") or latest.get("errors"):
                raise AssertionError(latest)
            if predicate(latest):
                return latest
        except (FileNotFoundError, json.JSONDecodeError):
            pass
        time.sleep(0.25)
    raise AssertionError(f"GNOME state timed out: {latest}")


def await_exit(pid: int) -> None:
    for _ in range(40):
        if not Path(f"/proc/{pid}").exists():
            return
        time.sleep(0.1)
    raise AssertionError(f"Renderer {pid} survived stop")


def inside_session(args) -> None:
    artifacts = Path(args.artifacts)
    runtime = Path(os.environ["XDG_RUNTIME_DIR"])
    command("gsettings", "set", "org.gnome.shell", "enabled-extensions", f"['{UUID}', '{PROBE}']")
    command("gsettings", "set", "org.gnome.shell", "disable-user-extensions", "false")
    command("gsettings", "set", SCHEMA, "renderer", str(Path(args.renderer).resolve()))
    command("gsettings", "set", SCHEMA, "scene", "matrix")
    command("gsettings", "set", SCHEMA, "theme", "amber")
    command("gsettings", "set", SCHEMA, "running", "true")
    read_fd, write_fd = os.pipe()
    display = subprocess.Popen(["Xvfb", "-displayfd", str(write_fd), "-screen", "0", "1280x900x24"],
                               pass_fds=(write_fd,), stderr=subprocess.DEVNULL)
    os.close(write_fd)
    with os.fdopen(read_fd) as pipe:
        os.environ["DISPLAY"] = ":" + pipe.readline().strip()
    log = (artifacts / "gnome-shell.log").open("w")
    shell = subprocess.Popen(["gnome-shell", "--nested", "--wayland", "--no-x11"],
                             stdout=log, stderr=log, start_new_session=True)
    try:
        first = await_report(runtime, lambda r: r.get("texture", 0) > 0 and r.get("background"))
        assert first["actors"] == first["monitors"]
        assert command("gnome-extensions", "list", "--enabled").splitlines().count(UUID) == 1
        time.sleep(2)
        command("import", "-window", "root", str(artifacts / "gnome-wayland-wallpaper.png"))
        old_pid = int(first["pid"])
        command("gsettings", "set", SCHEMA, "scene", "pipes")
        await_report(runtime, lambda r: r.get("texture", 0) > 0 and r.get("pid") != str(old_pid))
        await_exit(old_pid)
        command("gsettings", "set", SCHEMA, "running", "false")
        await_report(runtime, lambda r: r.get("pid") is None and r.get("actors") == 0)
        command("gsettings", "set", SCHEMA, "running", "true")
        restarted = await_report(runtime, lambda r: r.get("texture", 0) > 0)
        command("gnome-extensions", "disable", UUID)
        await_report(runtime, lambda r: r.get("pid") is None and r.get("actors") == 0)
        await_exit(int(restarted["pid"]))
        command("gnome-extensions", "enable", UUID)
        await_report(runtime, lambda r: r.get("texture", 0) > 0 and r.get("background"))
        command("gsettings", "set", SCHEMA, "renderer", "/nonexistent/ascii-arcade")
        await_report(runtime, lambda r: r.get("pid") is None and r.get("actors") == 0)
        assert command("gsettings", "get", SCHEMA, "running") == "false"
        (artifacts / "result.json").write_text(json.dumps({
            "status": "passed", "shell": command("gnome-shell", "--version"),
            "checks": ["frame upload", "background placement", "monitor actors", "scene switch",
                       "stop", "disable", "re-enable", "missing renderer", "process cleanup"],
        }, indent=2) + "\n")
        print("PASS: nested GNOME Wayland lifecycle and frame upload")
    finally:
        if shell.poll() is None:
            os.killpg(shell.pid, signal.SIGTERM)
            try:
                shell.wait(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(shell.pid, signal.SIGKILL)
                shell.wait()
        display.terminate()
        display.wait(timeout=10)
        log.close()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--renderer", required=True)
    parser.add_argument("--artifacts", required=True)
    parser.add_argument("--inside-session", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.inside_session:
        inside_session(args)
        return
    if not command("gnome-shell", "--version").startswith("GNOME Shell 46."):
        raise SystemExit("This integration check requires GNOME Shell 46.")
    artifacts = Path(args.artifacts).resolve()
    artifacts.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ascii-gnome-") as directory:
        work = Path(directory)
        runtime = work / "runtime"
        runtime.mkdir(mode=0o700)
        data = work / "data"
        extensions = data / "gnome-shell/extensions"
        extension = extensions / UUID
        shutil.copytree(ROOT / "shells/linux/gnome", extension)
        command("glib-compile-schemas", "--strict", str(extension / "schemas"))
        probe = extensions / PROBE
        probe.mkdir()
        (probe / "metadata.json").write_text(json.dumps({
            "uuid": PROBE, "name": "ASCII Arcade test probe", "description": "Isolated test observer",
            "shell-version": ["46"],
        }))
        (probe / "extension.js").write_text(PROBE_SOURCE)
        settings = dict(os.environ, XDG_DATA_HOME=str(data), XDG_CONFIG_HOME=str(work / "config"),
                        XDG_CACHE_HOME=str(work / "cache"), XDG_RUNTIME_DIR=str(runtime),
                        GSETTINGS_SCHEMA_DIR=str(extension / "schemas"),
                        LIBGL_ALWAYS_SOFTWARE="1", GSK_RENDERER="cairo")
        try:
            subprocess.run(["dbus-run-session", "--", sys.executable, str(Path(__file__).resolve()),
                            "--inside-session", "--renderer", str(Path(args.renderer).resolve()),
                            "--artifacts", str(artifacts)], env=settings, check=True, timeout=180)
        finally:
            unmount = shutil.which("fusermount3") or shutil.which("fusermount")
            if unmount:
                subprocess.run([unmount, "-u", str(runtime / "doc")],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10)


if __name__ == "__main__":
    main()
