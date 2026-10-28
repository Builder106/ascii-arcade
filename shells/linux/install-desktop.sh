#!/usr/bin/env bash
set -euo pipefail

app_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
data_dir=${XDG_DATA_HOME:-"$HOME/.local/share"}
destination="$data_dir/ascii-arcade"
if ! command -v zenity >/dev/null; then
    printf '%s\n' 'Install the zenity package before installing ASCII Arcade.' >&2
    exit 1
fi
mkdir -p -- "$destination" "$data_dir/applications"
for file in aa aa-linux desktop.sh icon.png; do
    if [[ "$app_dir/$file" != "$destination/$file" ]]; then
        install -m 644 -- "$app_dir/$file" "$destination/$file"
    fi
done
chmod +x -- "$destination/aa" "$destination/aa-linux" "$destination/desktop.sh"
if [[ -x "$app_dir/bin/doom_ascii" ]]; then
    install -D -m 755 -- "$app_dir/bin/doom_ascii" "$destination/bin/doom_ascii"
fi
if [[ -d "$app_dir/wad" ]]; then
    mkdir -p -- "$destination/wad"
    install -m 644 -- "$app_dir"/wad/*.wad "$destination/wad/"
fi
for file in doom-ascii-source.tar.gz LICENSES/doom-ascii.GPL-2.0; do
    if [[ -f "$app_dir/$file" ]]; then
        install -D -m 644 -- "$app_dir/$file" "$destination/$file"
    fi
done

if [[ -d "$app_dir/gnome" ]]; then
    extension_dir="$data_dir/gnome-shell/extensions/ascii-arcade@builder106"
    mkdir -p -- "$extension_dir/schemas"
    install -m 644 -- "$app_dir/gnome/extension.js" "$app_dir/gnome/metadata.json" "$extension_dir/"
    install -m 644 -- "$app_dir/gnome/schemas/org.gnome.shell.extensions.ascii-arcade.gschema.xml" "$extension_dir/schemas/"
    glib-compile-schemas --strict "$extension_dir/schemas"
    printf '%s\n' 'GNOME: log out and back in, then enable ASCII Arcade in the Extensions app.'
fi

exec_path="$destination/desktop.sh"
exec_path=${exec_path//\\/\\\\\\\\}
exec_path=${exec_path//\"/\\\\\"}
exec_path=${exec_path//\$/\\\\\$}
exec_path=${exec_path//\`/\\\\\`}
exec_path=${exec_path//%/%%}
icon_path=${destination//\\/\\\\}
cat >"$data_dir/applications/ascii-arcade.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=ASCII Arcade
Comment=Choose and control a live ASCII wallpaper
Exec="$exec_path"
Icon=$icon_path/icon.png
Terminal=false
Categories=Graphics;
EOF
printf '%s\n' 'Installed. Open ASCII Arcade from your application menu.'
