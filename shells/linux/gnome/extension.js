import Clutter from 'gi://Clutter';
import Cogl from 'gi://Cogl';
import Gio from 'gi://Gio';
import GioUnix from 'gi://GioUnix';
import GLib from 'gi://GLib';
import GLibUnix from 'gi://GLibUnix';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

export default class AsciiArcadeExtension extends Extension {
    enable() {
        this._settings = this.getSettings();
        this._run = null;
        this._group = null;
        this._button = new PanelMenu.Button(0.0, 'ASCII Arcade');
        this._button.add_child(new St.Label({text: 'ASCII Arcade', y_align: Clutter.ActorAlign.CENTER}));
        this._switch = new PopupMenu.PopupSwitchMenuItem('Wallpaper', this._settings.get_boolean('running'));
        this._switch.connect('toggled', (_item, value) => this._settings.set_boolean('running', value));
        this._button.menu.addMenuItem(this._switch);
        for (const [key, title, values] of [
            ['scene', 'Scene', ['donut', 'helix', 'matrix', 'pipes', 'life']],
            ['theme', 'Theme', ['hacker', 'amber', 'ice', 'ghost']],
        ]) {
            const submenu = new PopupMenu.PopupSubMenuMenuItem(title);
            for (const value of values)
                submenu.menu.addAction(value, () => this._settings.set_string(key, value));
            this._button.menu.addMenuItem(submenu);
        }
        Main.panel.addToStatusArea(this.uuid, this._button);
        this._settingsSignal = this._settings.connect('changed', () => this._sync());
        this._monitorSignal = Main.layoutManager.connect('monitors-changed', () => this._sync());
        this._sessionSignal = Main.sessionMode.connect('updated', () => this._sync());
        this._backgroundSignal = Main.layoutManager._backgroundGroup.connect('child-added', () => {
            if (this._group)
                Main.layoutManager._backgroundGroup.set_child_above_sibling(this._group, null);
        });
        this._sync();
    }

    _sync() {
        this._stop();
        const running = this._settings.get_boolean('running');
        this._switch.setToggleState(running);
        if (!running || Main.sessionMode.isLocked || Main.layoutManager.monitors.length === 0)
            return;
        try {
            const monitor = Main.layoutManager.primaryMonitor ?? Main.layoutManager.monitors[0];
            const scale = Math.min(1, 640 / monitor.width, 360 / monitor.height);
            const width = Math.max(8, Math.floor(monitor.width * scale / 8) * 8);
            const height = Math.max(16, Math.floor(monitor.height * scale / 16) * 16);
            const renderer = this._settings.get_string('renderer') ||
                GLib.build_filenamev([GLib.get_user_data_dir(), 'ascii-arcade', 'aa-linux']);
            const process = Gio.Subprocess.new([
                renderer, '--gnome-stream', this._settings.get_string('scene'),
                this._settings.get_string('theme'), String(width), String(height),
            ], Gio.SubprocessFlags.STDOUT_PIPE);
            const run = {
                process,
                stream: process.get_stdout_pipe(),
                channel: null,
                watchId: 0,
                header: new Uint8Array(12),
                headerOffset: 0,
                frame: null,
                frameOffset: 0,
            };
            this._run = run;
            this._image = new St.ImageContent({preferred_width: width, preferred_height: height});
            this._group = new Clutter.Actor({reactive: false});
            for (const output of Main.layoutManager.monitors) {
                this._group.add_child(new Clutter.Actor({
                    x: output.x, y: output.y, width: output.width, height: output.height,
                    content: this._image, reactive: false,
                }));
            }
            Main.layoutManager._backgroundGroup.add_child(this._group);
            const fd = GioUnix.FileDescriptorBased.get_fd(run.stream);
            if (!GLibUnix.set_fd_nonblocking(fd, true))
                throw new Error('Unable to set wallpaper renderer stream to non-blocking mode');
            run.channel = GLib.IOChannel.unix_new(fd);
            run.watchId = GLib.io_add_watch(
                run.channel,
                GLib.PRIORITY_DEFAULT,
                GLib.IOCondition.IN | GLib.IOCondition.HUP | GLib.IOCondition.ERR,
                () => this._readFrames(run, width, height),
            );
        } catch (error) {
            this._stop();
            this._settings.set_boolean('running', false);
            Main.notifyError('ASCII Arcade', error.message);
        }
    }

    _readFrames(run, width, height) {
        if (this._run !== run)
            return GLib.SOURCE_REMOVE;

        try {
            const bytes = run.stream.read_bytes(65536, null);
            if (bytes.get_size() === 0)
                throw new Error('Wallpaper renderer closed its frame stream');

            const chunk = bytes.get_data();
            let offset = 0;
            while (offset < chunk.length) {
                if (!run.frame) {
                    const count = Math.min(run.header.length - run.headerOffset, chunk.length - offset);
                    run.header.set(chunk.subarray(offset, offset + count), run.headerOffset);
                    run.headerOffset += count;
                    offset += count;
                    if (run.headerOffset < run.header.length)
                        continue;

                    const view = new DataView(run.header.buffer);
                    if (new TextDecoder().decode(run.header.subarray(0, 4)) !== 'AA01' ||
                        view.getUint32(4, true) !== width || view.getUint32(8, true) !== height)
                        throw new Error('Incompatible wallpaper renderer. Reinstall the desktop package.');
                    run.frame = new Uint8Array(width * height * 4);
                    continue;
                }

                const count = Math.min(run.frame.length - run.frameOffset, chunk.length - offset);
                run.frame.set(chunk.subarray(offset, offset + count), run.frameOffset);
                run.frameOffset += count;
                offset += count;
                if (run.frameOffset === run.frame.length) {
                    this._image.set_bytes(new GLib.Bytes(run.frame), Cogl.PixelFormat.RGBA_8888,
                        width, height, width * 4);
                    run.frame = new Uint8Array(width * height * 4);
                    run.frameOffset = 0;
                }
            }
            return GLib.SOURCE_CONTINUE;
        } catch (error) {
            if (this._run !== run)
                return GLib.SOURCE_REMOVE;
            if (error.matches?.(Gio.IOErrorEnum, Gio.IOErrorEnum.WOULD_BLOCK))
                return GLib.SOURCE_CONTINUE;
            run.watchId = 0;
            this._stop();
            this._settings.set_boolean('running', false);
            Main.notifyError('ASCII Arcade', error.message);
            return GLib.SOURCE_REMOVE;
        }
    }

    _stop() {
        const run = this._run;
        this._run = null;
        if (run) {
            if (run.watchId) {
                GLib.source_remove(run.watchId);
                run.watchId = 0;
            }
            run.process.force_exit();
            run.process.wait(null);
            run.stream.close(null);
            run.channel = null;
        }
        this._group?.destroy();
        this._group = null;
        this._image = null;
    }

    disable() {
        this._settings.disconnect(this._settingsSignal);
        Main.layoutManager.disconnect(this._monitorSignal);
        Main.sessionMode.disconnect(this._sessionSignal);
        Main.layoutManager._backgroundGroup.disconnect(this._backgroundSignal);
        this._stop();
        this._button.destroy();
        this._button = null;
        this._switch = null;
        this._settings = null;
    }
}
