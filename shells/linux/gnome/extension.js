import Clutter from 'gi://Clutter';
import Cogl from 'gi://Cogl';
import Gio from 'gi://Gio';
import GioUnix from 'gi://GioUnix';
import GLib from 'gi://GLib';
import GLibUnix from 'gi://GLibUnix';
import Meta from 'gi://Meta';
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
        this._idleWatchId = 0;
        this._activeWatchId = 0;
        this._idleCycleId = 0;
        this._idleOriginalScene = null;
        this._lastIdleSceneChange = 0;
        this._button = new PanelMenu.Button(0.0, 'ASCII Arcade');
        this._button.add_child(new St.Label({text: 'ASCII Arcade', y_align: Clutter.ActorAlign.CENTER}));
        this._switch = new PopupMenu.PopupSwitchMenuItem('Wallpaper', this._settings.get_boolean('running'));
        this._switch.connect('toggled', (_item, value) => this._settings.set_boolean('running', value));
        this._button.menu.addMenuItem(this._switch);
        this._doomSwitch = new PopupMenu.PopupSwitchMenuItem('Enable DOOM Scene', this._settings.get_boolean('doom-enabled'));
        this._doomSwitch.connect('toggled', (_item, enabled) => {
            this._settings.set_boolean('doom-enabled', enabled);
            if (!enabled && this._settings.get_string('scene') === 'doom')
                this._settings.set_string('scene', 'donut');
            this._refreshSceneMenu();
        });
        this._button.menu.addMenuItem(this._doomSwitch);
        this._opaqueSwitch = new PopupMenu.PopupSwitchMenuItem(
            'Opaque Background', this._settings.get_boolean('opaque-background'));
        this._opaqueSwitch.connect('toggled', (_item, enabled) =>
            this._settings.set_boolean('opaque-background', enabled));
        this._button.menu.addMenuItem(this._opaqueSwitch);
        this._captureSwitch = new PopupMenu.PopupSwitchMenuItem(
            'Capture keys for DOOM', this._settings.get_boolean('capture-keys-for-doom'));
        this._captureSwitch.connect('toggled', (_item, enabled) =>
            this._settings.set_boolean('capture-keys-for-doom', enabled));
        this._button.menu.addMenuItem(this._captureSwitch);
        this._idleSwitch = new PopupMenu.PopupSwitchMenuItem(
            'Auto-cycle when idle', this._settings.get_boolean('idle-cycle'));
        this._idleSwitch.connect('toggled', (_item, enabled) => {
            this._settings.set_boolean('idle-cycle', enabled);
            if (!enabled)
                this._endIdleCycle();
            this._armIdleWatch();
        });
        this._button.menu.addMenuItem(this._idleSwitch);
        this._loginSwitch = new PopupMenu.PopupSwitchMenuItem(
            'Launch at Login', this._settings.get_boolean('launch-at-login'));
        this._loginSwitch.connect('toggled', (_item, enabled) =>
            this._settings.set_boolean('launch-at-login', enabled));
        this._button.menu.addMenuItem(this._loginSwitch);
        for (const [key, title, values] of [
            ['scene', 'Scene', ['donut', 'helix', 'matrix', 'pipes', 'life']],
            ['theme', 'Theme', ['hacker', 'amber', 'ice', 'ghost']],
        ]) {
            const submenu = new PopupMenu.PopupSubMenuMenuItem(title);
            if (key === 'scene') {
                this._sceneMenu = submenu.menu;
                this._refreshSceneMenu();
            } else {
                for (const value of values)
                    submenu.menu.addAction(value, () => this._settings.set_string(key, value));
            }
            this._button.menu.addMenuItem(submenu);
        }
        const settingsMenu = new PopupMenu.PopupSubMenuMenuItem('Scene Settings');
        this._settingsOptionsMenu = settingsMenu.menu;
        this._button.menu.addMenuItem(settingsMenu);
        Main.panel.addToStatusArea(this.uuid, this._button);
        this._settingsSignal = this._settings.connect('changed', () => {
            if (!this._settings.get_boolean('idle-cycle'))
                this._endIdleCycle();
            this._refreshSceneMenu();
            this._refreshSettingsMenu();
            this._sync();
        });
        this._monitorSignal = Main.layoutManager.connect('monitors-changed', () => this._sync());
        this._sessionSignal = Main.sessionMode.connect('updated', () => this._sync());
        this._backgroundSignal = Main.layoutManager._backgroundGroup.connect('child-added', () => {
            if (this._group)
                Main.layoutManager._backgroundGroup.set_child_above_sibling(this._group, null);
        });
        this._idleMonitor = global.backend.get_core_idle_monitor();
        this._activeWatchId = this._idleMonitor.add_user_active_watch(() => {
            this._endIdleCycle();
            this._armIdleWatch();
        });
        if (this._settings.get_boolean('launch-at-login'))
            this._settings.set_boolean('running', true);
        this._keyEventSignal = global.stage.connect('captured-event', (_stage, event) =>
            this._captureDoomKey(event));
        this._sync();
        this._armIdleWatch();
        this._button.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        this._button.menu.addAction('Save Screenshot', () => this._capture('screenshot'));
        this._button.menu.addAction('Record 3-Sec Clip', () => this._capture('clip'));
    }

    _capture(kind) {
        const renderer = this._settings.get_string('renderer') ||
            GLib.build_filenamev([GLib.get_user_data_dir(), 'ascii-arcade', 'aa-linux']);
        const cli = GLib.build_filenamev([GLib.path_get_dirname(renderer), 'aa']);
        const args = [cli, 'capture', kind, this._settings.get_string('scene'),
            '--theme', this._settings.get_string('theme')];
        if (this._settings.get_boolean('doom-enabled'))
            args.push('--enable-doom');
        args.push(...this._sceneSettingArgs());
        try {
            const process = Gio.Subprocess.new(args,
                Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_PIPE);
            process.communicate_utf8_async(null, null, (source, result) => {
                try {
                    const [ok, stdout, stderr] = source.communicate_utf8_finish(result);
                    if (!ok || !source.get_successful())
                        throw new Error(stderr || 'Capture failed');
                    Main.notify(`ASCII Arcade: saved ${stdout.trim()}`);
                } catch (error) {
                    Main.notifyError('ASCII Arcade capture failed', error.message);
                }
            });
        } catch (error) {
            Main.notifyError('ASCII Arcade capture failed', error.message);
        }
    }

    _captureDoomKey(event) {
        if (!this._settings.get_boolean('capture-keys-for-doom') ||
            !this._settings.get_boolean('doom-enabled') ||
            this._settings.get_string('scene') !== 'doom' ||
            event.type() !== Clutter.EventType.KEY_PRESS || !this._run?.input)
            return Clutter.EVENT_PROPAGATE;

        const modifiers = event.get_state();
        if (modifiers & (Clutter.ModifierType.CONTROL_MASK |
            Clutter.ModifierType.ALT_MASK | Clutter.ModifierType.SUPER_MASK))
            return Clutter.EVENT_PROPAGATE;

        const key = event.get_key_symbol();
        const keys = new Map([
            [Clutter.KEY_Left, [0x1b, 0x5b, 0x44]],
            [Clutter.KEY_Up, [0x1b, 0x5b, 0x41]],
            [Clutter.KEY_Right, [0x1b, 0x5b, 0x43]],
            [Clutter.KEY_Down, [0x1b, 0x5b, 0x42]],
            [Clutter.KEY_Return, [0x0a]],
            [Clutter.KEY_KP_Enter, [0x0a]],
            [Clutter.KEY_Escape, [0x1b]],
            [Clutter.KEY_space, [0x20]],
            [Clutter.KEY_comma, [0x2c]],
            [Clutter.KEY_period, [0x2e]],
            [Clutter.KEY_bracketright, [0x5d]],
        ]);
        let bytes = keys.get(key);
        if (!bytes && key >= 0x21 && key <= 0x7e)
            bytes = [key >= 0x41 && key <= 0x5a ? key + 0x20 : key];
        if (bytes) {
            try {
                this._run.input.write_bytes(new GLib.Bytes(new Uint8Array(bytes)), null);
            } catch (error) {
                Main.notifyError('ASCII Arcade DOOM input failed', error.message);
            }
        }
        return Clutter.EVENT_PROPAGATE;
    }

    _refreshSettingsMenu() {
        if (!this._settingsOptionsMenu)
            return;
        this._settingsOptionsMenu.removeAll();
        const scene = this._settings.get_string('scene');
        const definitions = {
            matrix: [
                ['Speed', 'matrix-speed', [['Slow', 9], ['Normal', 16], ['Fast', 26]]],
                ['Density', 'matrix-density', [['Sparse', 0.45], ['Normal', 0.72], ['Dense', 0.95]]],
            ],
            life: [
                ['Speed', 'life-speed', [['Slow', 4], ['Normal', 9], ['Fast', 16]]],
                ['Cell size', 'life-size', [['Small', 2], ['Medium', 3], ['Large', 4]]],
            ],
            pipes: [
                ['Speed', 'pipes-speed', [['Slow', 12], ['Normal', 22], ['Fast', 40]]],
                ['Pipes', 'pipes-count', [['Few', 2], ['Some', 4], ['Many', 8]]],
            ],
        }[scene];
        if (!definitions) {
            this._settingsOptionsMenu.addAction('No settings for this scene', () => {});
            return;
        }
        for (const [label, key, options] of definitions) {
            const submenu = new PopupMenu.PopupSubMenuMenuItem(label);
            const selected = this._settings.get_double(key);
            for (const [name, value] of options) {
                const suffix = value === selected ? ' ✓' : '';
                submenu.menu.addAction(`${name}${suffix}`, () => this._settings.set_double(key, value));
            }
            this._settingsOptionsMenu.addMenuItem(submenu);
        }
    }

    _sceneSettingArgs() {
        const scene = this._settings.get_string('scene');
        const values = {
            matrix: ['speed', 'matrix-speed', 'density', 'matrix-density'],
            life: ['speed', 'life-speed', 'size', 'life-size'],
            pipes: ['speed', 'pipes-speed', 'pipes', 'pipes-count'],
        }[scene];
        if (!values)
            return [];
        return [
            `--setting=${values[0]}=${this._settings.get_double(values[1])}`,
            `--setting=${values[2]}=${this._settings.get_double(values[3])}`,
        ];
    }

    _armIdleWatch() {
        if (!this._idleMonitor || this._idleWatchId ||
            this._idleCycleId ||
            !this._settings.get_boolean('idle-cycle') ||
            !this._settings.get_boolean('running'))
            return;
        const remaining = Math.max(1, 90000 - this._idleMonitor.get_idletime());
        this._idleWatchId = this._idleMonitor.add_idle_watch(remaining, () => {
            this._idleWatchId = 0;
            this._beginIdleCycle();
        });
    }

    _beginIdleCycle() {
        if (!this._settings.get_boolean('idle-cycle') ||
            !this._settings.get_boolean('running') || Main.sessionMode.isLocked)
            return;
        if (this._idleOriginalScene === null) {
            this._idleOriginalScene = this._settings.get_string('scene');
            this._lastIdleSceneChange = GLib.get_monotonic_time();
        }
        if (this._idleCycleId)
            return;
        this._idleCycleId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 20, () => {
            if (!this._settings.get_boolean('idle-cycle') || Main.sessionMode.isLocked) {
                this._idleCycleId = 0;
                return GLib.SOURCE_REMOVE;
            }
            const now = GLib.get_monotonic_time();
            if (now - this._lastIdleSceneChange < 20000000)
                return GLib.SOURCE_CONTINUE;
            this._lastIdleSceneChange = now;
            const scenes = ['donut', 'helix', 'matrix', 'pipes', 'life'];
            if (this._settings.get_boolean('doom-enabled'))
                scenes.push('doom');
            const current = this._settings.get_string('scene');
            const index = scenes.indexOf(current);
            this._settings.set_string('scene', scenes[(index + 1) % scenes.length]);
            return GLib.SOURCE_CONTINUE;
        });
    }

    _endIdleCycle() {
        if (this._idleWatchId) {
            this._idleMonitor.remove_watch(this._idleWatchId);
            this._idleWatchId = 0;
        }
        if (this._idleCycleId) {
            GLib.source_remove(this._idleCycleId);
            this._idleCycleId = 0;
        }
        if (this._idleOriginalScene !== null) {
            const original = this._idleOriginalScene;
            this._idleOriginalScene = null;
            if (this._settings.get_string('scene') !== original)
                this._settings.set_string('scene', original);
        }
    }

    _sync() {
        this._stop();
        if (this._settings.get_string('scene') === 'doom' &&
            !this._settings.get_boolean('doom-enabled'))
            this._settings.set_string('scene', 'donut');
        const running = this._settings.get_boolean('running');
        this._switch.setToggleState(running);
        this._doomSwitch.setToggleState(this._settings.get_boolean('doom-enabled'));
        this._opaqueSwitch.setToggleState(this._settings.get_boolean('opaque-background'));
        this._captureSwitch.setToggleState(this._settings.get_boolean('capture-keys-for-doom'));
        this._captureSwitch.setSensitive(this._settings.get_boolean('doom-enabled'));
        this._idleSwitch.setToggleState(this._settings.get_boolean('idle-cycle'));
        this._loginSwitch.setToggleState(this._settings.get_boolean('launch-at-login'));
        this._refreshSettingsMenu();
        this._armIdleWatch();
        if (!running || Main.sessionMode.isLocked || Main.layoutManager.monitors.length === 0)
            return;
        try {
            const monitor = Main.layoutManager.primaryMonitor ?? Main.layoutManager.monitors[0];
            const scale = Math.min(1, 640 / monitor.width, 360 / monitor.height);
            const width = Math.max(8, Math.floor(monitor.width * scale / 8) * 8);
            const height = Math.max(16, Math.floor(monitor.height * scale / 16) * 16);
            const renderer = this._settings.get_string('renderer') ||
                GLib.build_filenamev([GLib.get_user_data_dir(), 'ascii-arcade', 'aa-linux']);
            const args = [
                renderer, '--gnome-stream', this._settings.get_string('scene'),
                this._settings.get_string('theme'), String(width), String(height),
            ];
            if (this._settings.get_boolean('doom-enabled'))
                args.push('--enable-doom');
            if (this._settings.get_boolean('idle-cycle'))
                args.push('--idle-cycle');
            if (this._settings.get_boolean('capture-keys-for-doom') &&
                this._settings.get_string('scene') === 'doom')
                args.push('--capture-keys');
            if (!this._settings.get_boolean('opaque-background'))
                args.push('--transparent');
            for (const setting of this._sceneSettingArgs())
                args.push(setting);
            let flags = Gio.SubprocessFlags.STDOUT_PIPE;
            const captureKeys = args.includes('--capture-keys');
            if (captureKeys)
                flags |= Gio.SubprocessFlags.STDIN_PIPE;
            const process = Gio.Subprocess.new(args, flags);
            const run = {
                process,
                input: captureKeys ? process.get_stdin_pipe() : null,
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

    _refreshSceneMenu() {
        if (!this._sceneMenu)
            return;
        this._sceneMenu.removeAll();
        const scenes = ['donut', 'helix', 'matrix', 'pipes', 'life'];
        if (this._settings.get_boolean('doom-enabled'))
            scenes.push('doom');
        for (const scene of scenes)
            this._sceneMenu.addAction(scene, () => this._settings.set_string('scene', scene));
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
            run.input?.close(null);
            run.stream.close(null);
            run.channel = null;
        }
        this._group?.destroy();
        this._group = null;
        this._image = null;
    }

    disable() {
        this._endIdleCycle();
        if (this._activeWatchId) {
            this._idleMonitor.remove_watch(this._activeWatchId);
            this._activeWatchId = 0;
        }
        if (this._keyEventSignal) {
            global.stage.disconnect(this._keyEventSignal);
            this._keyEventSignal = 0;
        }
        this._settings.disconnect(this._settingsSignal);
        Main.layoutManager.disconnect(this._monitorSignal);
        Main.sessionMode.disconnect(this._sessionSignal);
        Main.layoutManager._backgroundGroup.disconnect(this._backgroundSignal);
        this._stop();
        this._button.destroy();
        this._button = null;
        this._switch = null;
        this._idleMonitor = null;
        this._settings = null;
    }
}
