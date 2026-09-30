#!/usr/bin/env bash
# Extrai/injeta o EFI/OC/config.plist de dentro de uma imagem OpenCore.qcow2.
# Requer libguestfs-tools (guestfish). Rode numa Debian.
#
#   oc-image.sh extract <img.qcow2> <config.plist>   # img → arquivo
#   oc-image.sh inject  <img.qcow2> <config.plist>   # arquivo → img
#
# A partição/caminho do ESP dentro da imagem são configuráveis:
#   OC_PART (default /dev/sda1), OC_PLIST_PATH (default /EFI/OC/config.plist)
set -euo pipefail

cmd="${1:-}"; img="${2:-}"; plist="${3:-}"
OC_PART="${OC_PART:-/dev/sda1}"
OC_PLIST_PATH="${OC_PLIST_PATH:-/EFI/OC/config.plist}"

usage() { echo "uso: oc-image.sh <extract|inject> <img.qcow2> <config.plist>" >&2; exit 2; }
[ -n "$cmd" ] && [ -n "$img" ] && [ -n "$plist" ] || usage

case "$cmd" in
	extract)
		guestfish --ro -a "$img" run : mount "$OC_PART" / : download "$OC_PLIST_PATH" "$plist"
		;;
	inject)
		guestfish --rw -a "$img" run : mount "$OC_PART" / : upload "$plist" "$OC_PLIST_PATH"
		;;
	*)
		usage
		;;
esac
