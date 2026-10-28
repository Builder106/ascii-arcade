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
settings_args=()
matrix_settings=()
life_settings=()
pipes_settings=()
doom_enabled=false
opaque_background=true
idle_cycle=false
capture_keys=true
state_file="${XDG_CONFIG_HOME:-$HOME/.config}/ascii-arcade/desktop.conf"
load_state() {
    [[ -r "$state_file" ]] || return 0
    while IFS='=' read -r key value; do
        case "$key" in
            scene) case "$value" in donut|helix|matrix|life|pipes|doom) scene=$value ;; esac ;;
            theme) case "$value" in hacker|amber|ice|ghost) theme=$value ;; esac ;;
            doom_enabled|opaque_background|idle_cycle|capture_keys)
                case "$value" in true|false) printf -v "$key" '%s' "$value" ;; esac
                ;;
            matrix_settings) IFS='|' read -r -a matrix_settings <<<"$value" ;;
            life_settings) IFS='|' read -r -a life_settings <<<"$value" ;;
            pipes_settings) IFS='|' read -r -a pipes_settings <<<"$value" ;;
        esac
    done < "$state_file"
}
save_state() {
    mkdir -p -- "$(dirname -- "$state_file")"
    local temp_file="$state_file.tmp.$$"
    {
        printf 'scene=%s\n' "$scene"
        printf 'theme=%s\n' "$theme"
        printf 'doom_enabled=%s\n' "$doom_enabled"
        printf 'opaque_background=%s\n' "$opaque_background"
        printf 'idle_cycle=%s\n' "$idle_cycle"
        printf 'capture_keys=%s\n' "$capture_keys"
        printf 'matrix_settings=%s\n' "$(IFS='|'; echo "${matrix_settings[*]}")"
        printf 'life_settings=%s\n' "$(IFS='|'; echo "${life_settings[*]}")"
        printf 'pipes_settings=%s\n' "$(IFS='|'; echo "${pipes_settings[*]}")"
    } > "$temp_file"
    chmod 600 "$temp_file"
    mv -f -- "$temp_file" "$state_file"
}
load_scene_settings() {
    settings_args=()
    case "$scene" in
        matrix) settings_args=("${matrix_settings[@]}") ;;
        life) settings_args=("${life_settings[@]}") ;;
        pipes) settings_args=("${pipes_settings[@]}") ;;
    esac
}
store_scene_settings() {
    case "$scene" in
        matrix) matrix_settings=("${settings_args[@]}") ;;
        life) life_settings=("${settings_args[@]}") ;;
        pipes) pipes_settings=("${settings_args[@]}") ;;
    esac
}
load_state
if "$gnome" && [[ -f "$schema_dir/gschemas.compiled" ]]; then
    scene=$(gsettings --schemadir="$schema_dir" get org.gnome.shell.extensions.ascii-arcade scene | tr -d "'")
    theme=$(gsettings --schemadir="$schema_dir" get org.gnome.shell.extensions.ascii-arcade theme | tr -d "'")
    doom_enabled=$(gsettings --schemadir="$schema_dir" get org.gnome.shell.extensions.ascii-arcade doom-enabled)
    opaque_background=$(gsettings --schemadir="$schema_dir" get org.gnome.shell.extensions.ascii-arcade opaque-background)
    idle_cycle=$(gsettings --schemadir="$schema_dir" get org.gnome.shell.extensions.ascii-arcade idle-cycle)
    capture_keys=$(gsettings --schemadir="$schema_dir" get org.gnome.shell.extensions.ascii-arcade capture-keys-for-doom)
    login_at_start=$(gsettings --schemadir="$schema_dir" get org.gnome.shell.extensions.ascii-arcade launch-at-login)
    case "$scene" in
        matrix) settings_args=("--setting=speed=$(gsettings --schemadir="$schema_dir" get org.gnome.shell.extensions.ascii-arcade matrix-speed | tr -d ' ')" "--setting=density=$(gsettings --schemadir="$schema_dir" get org.gnome.shell.extensions.ascii-arcade matrix-density | tr -d ' ')") ;;
        life) settings_args=("--setting=speed=$(gsettings --schemadir="$schema_dir" get org.gnome.shell.extensions.ascii-arcade life-speed | tr -d ' ')" "--setting=size=$(gsettings --schemadir="$schema_dir" get org.gnome.shell.extensions.ascii-arcade life-size | tr -d ' ')") ;;
        pipes) settings_args=("--setting=speed=$(gsettings --schemadir="$schema_dir" get org.gnome.shell.extensions.ascii-arcade pipes-speed | tr -d ' ')" "--setting=pipes=$(gsettings --schemadir="$schema_dir" get org.gnome.shell.extensions.ascii-arcade pipes-count | tr -d ' ')") ;;
    esac
elif [[ -x "$app_dir/aa-linux" ]]; then
    login_at_start=$("$app_dir/aa-linux" --autostart-status)
else
    login_at_start=false
fi
if ! "$gnome"; then load_scene_settings; fi
while true; do
    action=$(zenity --list --title='ASCII Arcade' --width=460 --height=430 \
        --text="Wallpaper: $scene / $theme. DOOM: $doom_enabled. Capture keys: $capture_keys. Idle cycle: $idle_cycle. Opaque: $opaque_background. Launch at login: $login_at_start. Closing this controller stops its wallpaper." \
        --column=Action 'Start wallpaper' 'Choose scene' 'Choose theme' 'Scene Settings' 'Enable or disable DOOM' 'Toggle capture keys for DOOM' 'Toggle Auto-cycle when idle' 'Toggle opaque background' 'Toggle Launch at Login' 'Save Screenshot' 'Record 3-Sec Clip' 'Stop wallpaper' 'Quit') || break
    case "$action" in
        'Choose scene')
            scenes=(donut helix matrix life pipes)
            if "$doom_enabled"; then scenes+=(doom); fi
            if choice=$(zenity --list --title='Choose scene' --column=Scene \
                "${scenes[@]}") && [[ -n "$choice" ]]; then
                scene=$choice
                load_scene_settings
            fi
            ;;
        'Choose theme')
            if choice=$(zenity --list --title='Choose theme' --column=Theme \
                hacker amber ice ghost) && [[ -n "$choice" ]]; then
                theme=$choice
            fi
            ;;
        'Scene Settings')
            case "$scene" in
                matrix)
                    if form=$(zenity --forms --title='Matrix Settings' --separator='|' \
                        --add-combo='Speed' --combo-values='Slow|Normal|Fast' \
                        --add-combo='Density' --combo-values='Sparse|Normal|Dense'); then
                        IFS='|' read -r speed density <<<"$form"
                        case "$speed" in Slow) speed_value=9 ;; Normal) speed_value=16 ;; Fast) speed_value=26 ;; esac
                        case "$density" in Sparse) density_value=0.45 ;; Normal) density_value=0.72 ;; Dense) density_value=0.95 ;; esac
                        settings_args=("--setting=speed=$speed_value" "--setting=density=$density_value")
                        store_scene_settings
                        if "$gnome"; then
                            gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade matrix-speed "$speed_value"
                            gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade matrix-density "$density_value"
                        fi
                    fi
                    ;;
                life)
                    if form=$(zenity --forms --title='Life Settings' --separator='|' \
                        --add-combo='Speed' --combo-values='Slow|Normal|Fast' \
                        --add-combo='Cell size' --combo-values='Small|Medium|Large'); then
                        IFS='|' read -r speed size <<<"$form"
                        case "$speed" in Slow) speed_value=4 ;; Normal) speed_value=9 ;; Fast) speed_value=16 ;; esac
                        case "$size" in Small) size_value=2 ;; Medium) size_value=3 ;; Large) size_value=4 ;; esac
                        settings_args=("--setting=speed=$speed_value" "--setting=size=$size_value")
                        store_scene_settings
                        if "$gnome"; then
                            gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade life-speed "$speed_value"
                            gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade life-size "$size_value"
                        fi
                    fi
                    ;;
                pipes)
                    if form=$(zenity --forms --title='Pipes Settings' --separator='|' \
                        --add-combo='Speed' --combo-values='Slow|Normal|Fast' \
                        --add-combo='Pipes' --combo-values='Few|Some|Many'); then
                        IFS='|' read -r speed count <<<"$form"
                        case "$speed" in Slow) speed_value=12 ;; Normal) speed_value=22 ;; Fast) speed_value=40 ;; esac
                        case "$count" in Few) count_value=2 ;; Some) count_value=4 ;; Many) count_value=8 ;; esac
                        settings_args=("--setting=speed=$speed_value" "--setting=pipes=$count_value")
                        store_scene_settings
                        if "$gnome"; then
                            gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade pipes-speed "$speed_value"
                            gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade pipes-count "$count_value"
                        fi
                    fi
                    ;;
                *) zenity --info --text='This scene has no settings.' ;;
            esac
            ;;
        'Enable or disable DOOM')
            if "$doom_enabled"; then
                doom_enabled=false
                if [[ "$scene" == doom ]]; then scene=donut; fi
            else
                doom_enabled=true
            fi
            if "$gnome"; then
                gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade doom-enabled "$doom_enabled"
            fi
            ;;
        'Toggle capture keys for DOOM')
            if "$capture_keys"; then capture_keys=false; else capture_keys=true; fi
            if "$gnome"; then
                gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade capture-keys-for-doom "$capture_keys"
            fi
            ;;
        'Toggle Auto-cycle when idle')
            if "$idle_cycle"; then idle_cycle=false; else idle_cycle=true; fi
            if "$gnome"; then
                gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade idle-cycle "$idle_cycle"
            fi
            ;;
        'Toggle opaque background')
            if "$opaque_background"; then opaque_background=false; else opaque_background=true; fi
            if "$gnome"; then
                gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade opaque-background "$opaque_background"
            fi
            ;;
        'Toggle Launch at Login')
            if "$gnome"; then
                if "$login_at_start"; then login_at_start=false; else login_at_start=true; fi
                gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade launch-at-login "$login_at_start"
            elif "$login_at_start"; then
                "$app_dir/aa-linux" --autostart-disable
                login_at_start=false
            else
                login_args=(--autostart-enable "$scene" "$theme")
                if "$doom_enabled"; then login_args+=(--enable-doom); fi
                if ! "$opaque_background"; then login_args+=(--transparent); fi
                if "$idle_cycle"; then login_args+=(--idle-cycle); fi
                if "$capture_keys" && "$doom_enabled"; then login_args+=(--capture-keys); fi
                login_args+=("${settings_args[@]}")
                "$app_dir/aa-linux" "${login_args[@]}"
                login_at_start=true
            fi
            ;;
        'Save Screenshot')
            capture_args=(capture screenshot "$scene" --theme "$theme")
            if "$doom_enabled"; then capture_args+=(--enable-doom); fi
            capture_args+=("${settings_args[@]}")
            if path=$("$app_dir/aa" "${capture_args[@]}"); then
                if command -v wl-copy >/dev/null; then
                    wl-copy --type image/png < "$path"
                elif command -v xclip >/dev/null; then
                    xclip -selection clipboard -t image/png -i "$path"
                fi
                zenity --info --text="Screenshot saved to $path"
            else
                zenity --error --text='The screenshot could not be saved.'
            fi
            ;;
        'Record 3-Sec Clip')
            capture_args=(capture clip "$scene" --theme "$theme")
            if "$doom_enabled"; then capture_args+=(--enable-doom); fi
            capture_args+=("${settings_args[@]}")
            if path=$("$app_dir/aa" "${capture_args[@]}"); then
                zenity --info --text="Clip saved to $path"
            else
                zenity --error --text='The clip could not be saved.'
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
                gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade doom-enabled "$doom_enabled"
                gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade opaque-background "$opaque_background"
                gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade idle-cycle "$idle_cycle"
                gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade capture-keys-for-doom "$capture_keys"
                for setting in "${settings_args[@]}"; do
                    pair=${setting#--setting=}
                    id=${pair%%=*}
                    value=${pair#*=}
                    case "$scene:$id" in
                        matrix:speed) key=matrix-speed ;;
                        matrix:density) key=matrix-density ;;
                        life:speed) key=life-speed ;;
                        life:size) key=life-size ;;
                        pipes:speed) key=pipes-speed ;;
                        pipes:pipes) key=pipes-count ;;
                        *) continue ;;
                    esac
                    gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade "$key" "$value"
                done
                gsettings --schemadir="$schema_dir" set org.gnome.shell.extensions.ascii-arcade running true
                gnome_started=true
                continue
            fi
            doom_args=()
            if "$doom_enabled"; then doom_args+=(--enable-doom); fi
            if ! "$opaque_background"; then doom_args+=(--transparent); fi
            if "$idle_cycle"; then doom_args+=(--idle-cycle); fi
            if "$capture_keys" && "$doom_enabled"; then doom_args+=(--capture-keys); fi
            doom_args+=("${settings_args[@]}")
            "$app_dir/aa-linux" "$scene" "$theme" "${doom_args[@]}" >"$log_file" 2>&1 &
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
    save_state
done
