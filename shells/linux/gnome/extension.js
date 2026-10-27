import Clutter from 'gi://Clutter';
import Cogl from 'gi://Cogl';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

async function readExactly(stream, count, cancellable) {
    const data = new Uint8Array(count);
    let offset = 0;
    while (offset < count) {
        const bytes = await new Promise((resolve, reject) => {
            stream.read_bytes_async(count - offset, GLib.PRIORITY_DEFAULT, cancellable,
                (source, result) => {
                    try { resolve(source.read_bytes_finish(result)); }
                    catch (error) { reject(error); }
                });
        });
        if (bytes.get_size() === 0)
            throw new Error('Wallpaper renderer closed its frame stream');
        const chunk = bytes.get_data();
        data.set(chunk, offset);
        offset += chunk.length;
    }
    return data;
}

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
            const run = {process, stream: process.get_stdout_pipe(), cancellable: new Gio.Cancellable()};
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
            process.wait_async(null, (source, result) => {
                try { source.wait_finish(result); }
                catch (error) { console.error(`ASCII Arcade: ${error.message}`); }
            });
            this._readFrames(run, width, height).catch(error => {
                if (this._run !== run)
                    return;
                this._stop();
                this._settings.set_boolean('running', false);
                Main.notifyError('ASCII Arcade', error.message);
            }).finally(() => {
                run.stream.close_async(GLib.PRIORITY_DEFAULT, null, (source, result) => {
                    try { source.close_finish(result); }
                    catch (error) { console.error(`ASCII Arcade: ${error.message}`); }
                });
            });
        } catch (error) {
            this._stop();
            this._settings.set_boolean('running', false);
            Main.notifyError('ASCII Arcade', error.message);
        }
    }

    async _readFrames(run, width, height) {
        const header = await readExactly(run.stream, 12, run.cancellable);
        const view = new DataView(header.buffer);
        if (new TextDecoder().decode(header.subarray(0, 4)) !== 'AA01' ||
            view.getUint32(4, true) !== width || view.getUint32(8, true) !== height)
            throw new Error('Incompatible wallpaper renderer. Reinstall the desktop package.');
        while (this._run === run) {
            const pixels = await readExactly(run.stream, width * height * 4, run.cancellable);
            if (this._run !== run)
                return;
            this._image.set_bytes(new GLib.Bytes(pixels), Cogl.PixelFormat.RGBA_8888,
                width, height, width * 4);
        }
    }

    _stop() {
        const run = this._run;
        this._run = null;
        if (run) {
            run.cancellable.cancel();
            run.process.force_exit();
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
