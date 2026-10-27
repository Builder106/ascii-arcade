#!/usr/bin/env bash
set -euo pipefail

app_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
if ! command -v zenity >/dev/null; then
    printf '%s\n' 'ASCII Arcade requires Zenity. Install the zenity package and try again.' >&2
    exit 1
fi
if [[ ! -x "$app_dir/aa-linux" ]]; then
    zenity --error --text='The aa-linux executable is missing. Extract the whole desktop archive first.'
    exit 1
fi

wallpaper_pid=
gnome_started=false
gnome=false
schema_dir="${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions/ascii-arcade@builder106/schemas"
current_desktop=${XDG_CURRENT_DESKTOP:-}
if [[ -n "${WAYLAND_DISPLAY:-}" && "${current_desktop^^}" == *GNOME* ]]; then
    gnome=true
fi
log_file=$(mktemp)
stop_wallpaper() {
    if "$gnome_started"; then
        gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade running false
        gnome_started=false
    fi
    if [[ -n "$wallpaper_pid" ]]; then
        kill "$wallpaper_pid" 2>/dev/null || true
        wait "$wallpaper_pid" 2>/dev/null || true
        wallpaper_pid=
    fi
}
cleanup() {
    stop_wallpaper
    rm -f -- "$log_file"
}
trap cleanup EXIT
trap 'exit 0' INT TERM

scene=donut
theme=hacker
while true; do
    action=$(zenity --list --title='ASCII Arcade' --width=460 --height=330 \
        --text="Wallpaper: $scene / $theme. Closing this controller stops its wallpaper." \
        --column=Action 'Start wallpaper' 'Choose scene' 'Choose theme' 'Stop wallpaper' 'Quit') || break
    case "$action" in
        'Choose scene')
            if choice=$(zenity --list --title='Choose scene' --column=Scene \
                donut helix matrix life pipes) && [[ -n "$choice" ]]; then
                scene=$choice
            fi
            ;;
        'Choose theme')
            if choice=$(zenity --list --title='Choose theme' --column=Theme \
                hacker amber ice ghost) && [[ -n "$choice" ]]; then
                theme=$choice
            fi
            ;;
        'Start wallpaper')
            stop_wallpaper
            if "$gnome"; then
                enabled=false
                while IFS= read -r extension; do
                    if [[ "$extension" == 'ascii-arcade@builder106' ]]; then enabled=true; fi
                done < <(gnome-extensions list --enabled)
                if ! "$enabled" || [[ ! -f "$schema_dir/gschemas.compiled" ]]; then
                    zenity --error --text='Install the desktop package, then enable ASCII Arcade in GNOME Extensions. Log out and back in after the first installation.'
                    continue
                fi
                gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade scene "$scene"
                gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade theme "$theme"
                gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade running true
                gnome_started=true
                continue
            fi
            "$app_dir/aa-linux" "$scene" "$theme" >"$log_file" 2>&1 &
            wallpaper_pid=$!
            sleep 1
            if ! kill -0 "$wallpaper_pid" 2>/dev/null; then
                wait "$wallpaper_pid" 2>/dev/null || true
                wallpaper_pid=
                zenity --text-info --title='Wallpaper could not start' --filename="$log_file" \
                    --width=600 --height=300
            fi
            ;;
        'Stop wallpaper') stop_wallpaper ;;
        *) break ;;
    esac
done
