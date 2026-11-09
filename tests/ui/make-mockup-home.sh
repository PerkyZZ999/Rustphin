#!/usr/bin/env bash
# Fills an isolated home directory with the scene the HTML mockups in
# docs/mockup/ draw: the usual XDG folders listed as places, and a Pictures
# folder with the mockup's entries and dates. Used to compare the running app
# with the mockups (docs/project/UI_SCENARIOS.md, "Mockup comparison").
# Usage: tests/ui/make-mockup-home.sh <isolated-home>
# Never point it at a real home: it writes the places file.
set -euo pipefail

home="${1:?usage: make-mockup-home.sh <isolated-home>}"
case "$home" in
    /tmp/*) ;;
    *) echo "refusing to write outside /tmp: $home" >&2; exit 1 ;;
esac
if [[ -e "$home/Pictures" ]]; then
    echo "already filled: $home" >&2
    exit 1
fi
cd "$home"

mkdir -p Desktop Documents Downloads Pictures Music Videos .local/share
mkdir -p Downloads/Installers Documents/Notes

# Places, in the mockup's order. Trash comes from Rustphin itself.
# KDE writes each built-in place with its icon name, which picks the icon.
place() {
    printf ' <bookmark href="%s">\n  <title>%s</title>\n  <info>\n   <metadata owner="http://freedesktop.org">\n    <bookmark:icon name="%s"/>\n   </metadata>\n   <metadata owner="http://www.kde.org">\n    <ID>%s</ID>\n    <isSystemItem>true</isSystemItem>\n   </metadata>\n  </info>\n </bookmark>\n' "$1" "$2" "$3" "$4"
}
{
    printf '<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE xbel>\n'
    printf '<xbel xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks" xmlns:kdepriv="http://www.kde.org/kdepriv" xmlns:mime="http://www.freedesktop.org/standards/shared-mime-info">\n'
    place "file://$home" Home user-home 1/0
    place "file://$home/Desktop" Desktop user-desktop 1/1
    place "file://$home/Documents" Documents folder-documents 1/2
    place "file://$home/Downloads" Downloads folder-downloads 1/3
    place "file://$home/Pictures" Pictures folder-pictures 1/4
    place "file://$home/Music" Music folder-music 1/5
    place "file://$home/Videos" Videos folder-videos 1/6
    place "trash:/" Trash user-trash 1/7
    printf '</xbel>\n'
} > .local/share/user-places.xbel

cd Pictures
mkdir Projects Screenshots Wallpapers
# Five photographs: dusk gradients with a horizon, like the mockup's thumbnails.
colours=("#1b2a4a-#c47a5a" "#20304f-#e0a070" "#15233d-#8fb0d8" "#2a2240-#d08aa0" "#102030-#6aa0c0")
for i in 0 1 2 3 4; do
    name="IMG_48$((19 + i)).jpg"
    magick -size 1500x1000 "gradient:${colours[$i]}" \
        -fill '#0b1220' -draw "rectangle 0,700 1500,1000" \
        +noise Gaussian -attenuate 0.4 -quality 92 "$name"
    touch -d "2026-10-27 16:$((41 + i))" "$name"
done
magick -size 1200x800 "gradient:#3b5bd9-#8b5cf6" rustphin-design.png
printf 'Shots to sort.\n' > notes.txt
head -c 65536 /dev/urandom > archive.bin
python3 -I -c 'import sys, zipfile; z = zipfile.ZipFile(sys.argv[1], "w"); z.write(sys.argv[2], "archive.bin"); z.close()' archive.zip archive.bin
rm archive.bin
printf '%%PDF-1.4\n%%%%EOF\n' > presentation.pdf

touch -d "2026-11-04 20:18" rustphin-design.png
touch -d "2026-10-30 11:02" notes.txt
touch -d "2026-10-20 08:55" archive.zip
touch -d "2026-10-18 15:09" presentation.pdf
touch -d "2026-11-05 14:21" Projects
touch -d "2026-11-05 09:17" Screenshots
touch -d "2026-10-28 18:06" Wallpapers

echo "mockup scene ready in $home"
