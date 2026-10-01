#!/usr/bin/env bash
# Constrói a ISO do OSX-REIMS-OS com live-build (rode numa Debian 13).
#
# Requisitos: live-build, cargo (Rust), e root/sudo para `lb build`.
#   sudo apt install -y live-build
#
# Passos:
#   1. compila o reimsctl (release);
#   2. monta os binários em image/config/includes.chroot/usr/local/bin/;
#   3. (opcional) implanta o reims-vgpu pré-compilado em includes.chroot/opt/;
#   4. roda o live-build → ISO em image/live-image-amd64.hybrid.iso.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
img="${root}/image"
bin_dst="${img}/config/includes.chroot/usr/local/bin"

echo ">> 1/4 compilando reimsctl (release)"
( cd "${root}/reimsctl" && cargo build --release )

echo ">> 2/4 montando binários e helpers em includes.chroot"
mkdir -p "${bin_dst}"
cp "${root}/reimsctl/target/release/reimsctl" "${bin_dst}/reimsctl"
cp "${root}/provisioner/provision.sh" "${bin_dst}/provision.sh"
chmod +x "${bin_dst}/reimsctl" "${bin_dst}/provision.sh" "${bin_dst}/reims-console"

# helper de imagem OpenCore → /usr/local/lib/reims/oc-image.sh
lib_dst="${img}/config/includes.chroot/usr/local/lib/reims"
mkdir -p "${lib_dst}"
cp "${root}/opencore/oc-image.sh" "${lib_dst}/oc-image.sh"
chmod +x "${lib_dst}/oc-image.sh"

# bases de OpenCore (se já buscadas) → /usr/local/share/reims/opencore/<rail>/
if [ -d "${root}/opencore/base" ]; then
	share_dst="${img}/config/includes.chroot/usr/local/share/reims/opencore"
	mkdir -p "${share_dst}"
	cp -r "${root}/opencore/base/." "${share_dst}/"
fi

# 3/4 reims-vgpu pré-compilado (opcional): se existir vgpu/build/, copie para /opt
if [ -d "${root}/vgpu/build" ]; then
	echo ">> 3/4 implantando reims-vgpu pré-compilado"
	mkdir -p "${img}/config/includes.chroot/opt/reims-vgpu"
	cp -r "${root}/vgpu/build/." "${img}/config/includes.chroot/opt/reims-vgpu/"
else
	echo ">> 3/4 (pulando reims-vgpu — vgpu/build/ ausente; ver vgpu/README.md)"
fi

# SKIP_LB=1: só compila/monta o staging (o CI roda o `lb build` num container
# debian:trixie para ter o live-build correto da própria Debian).
if [ "${SKIP_LB:-0}" = "1" ]; then
	echo ">> SKIP_LB=1: staging pronto; pulando o lb build"
	exit 0
fi

echo ">> 4/4 live-build"
cd "${img}"
if [ "$(id -u)" -ne 0 ]; then
	SUDO="sudo"
else
	SUDO=""
fi
${SUDO} lb clean --purge || true
./auto/config
${SUDO} lb build

echo ">> pronto: procure a .iso em ${img}"
ls -lh "${img}"/*.iso 2>/dev/null || true
