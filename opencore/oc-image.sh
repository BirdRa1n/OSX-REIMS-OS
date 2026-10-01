#!/usr/bin/env bash
# Extrai/injeta o EFI/OC/config.plist de dentro de uma imagem OpenCore.qcow2.
# Usa qemu-nbd (do pacote qemu-utils) — sem libguestfs. Requer root + módulo nbd.
#
#   oc-image.sh extract <img.qcow2> <config.plist>   # img → arquivo
#   oc-image.sh inject  <img.qcow2> <config.plist>   # arquivo → img
#
# Config: NBD_DEV (default /dev/nbd0), OC_PARTNO (partição do ESP, default 1),
#         OC_PLIST_PATH (default /EFI/OC/config.plist)
set -euo pipefail

cmd="${1:-}"; img="${2:-}"; plist="${3:-}"
NBD_DEV="${NBD_DEV:-/dev/nbd0}"
OC_PARTNO="${OC_PARTNO:-1}"
OC_PLIST_PATH="${OC_PLIST_PATH:-/EFI/OC/config.plist}"

usage() { echo "uso: oc-image.sh <extract|inject> <img.qcow2> <config.plist>" >&2; exit 2; }
[ -n "$cmd" ] && [ -n "$img" ] && [ -n "$plist" ] || usage
[ "$(id -u)" -eq 0 ] || { echo "oc-image.sh: precisa de root (qemu-nbd/mount)" >&2; exit 1; }

part="${NBD_DEV}p${OC_PARTNO}"
mnt="$(mktemp -d)"

cleanup() {
	mountpoint -q "$mnt" && umount "$mnt" || true
	qemu-nbd --disconnect "$NBD_DEV" >/dev/null 2>&1 || true
	rmdir "$mnt" 2>/dev/null || true
}
trap cleanup EXIT

modprobe nbd max_part=8 2>/dev/null || true

case "$cmd" in
	extract)
		qemu-nbd --read-only --connect="$NBD_DEV" "$img"
		for _ in $(seq 1 20); do [ -b "$part" ] && break; sleep 0.3; done
		mount -o ro "$part" "$mnt"
		cp "${mnt}${OC_PLIST_PATH}" "$plist"
		;;
	inject)
		qemu-nbd --connect="$NBD_DEV" "$img"
		for _ in $(seq 1 20); do [ -b "$part" ] && break; sleep 0.3; done
		mount "$part" "$mnt"
		cp "$plist" "${mnt}${OC_PLIST_PATH}"
		sync
		;;
	*)
		usage
		;;
esac
