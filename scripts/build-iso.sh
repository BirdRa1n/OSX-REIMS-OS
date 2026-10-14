#!/usr/bin/env bash
# Constrói a ISO do OSX-REIMS-OS com live-build (rode numa Debian 13).
#
# Requisitos: live-build, cargo (Rust), e root/sudo para `lb build`.
#   sudo apt install -y live-build
#
# Passos:
#   1. compila o reimsctl (release);
#   2. monta o binário e a versão em image/config/includes.chroot/;
#   3. roda o live-build → ISO em image/*.iso.
#
# A ISO não traz o Reims vGPU nem drivers de GPU: o first setup e o gerenciador
# os baixam da origem e os compilam/instalam na máquina.
#
# SKIP_LB=1: só os passos 1–2 (o CI roda o `lb build` num container debian:trixie).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
img="${root}/image"
inc="${img}/config/includes.chroot"

version="$(sed -n 's/^version = "\(.*\)"/\1/p' "${root}/reimsctl/Cargo.toml" | head -n1)"
[ -n "${version}" ] || { echo "no version in reimsctl/Cargo.toml" >&2; exit 1; }

echo ">> 1/3 compilando reimsctl ${version} (release)"
( cd "${root}/reimsctl" && cargo build --release --locked )

echo ">> 2/3 montando o binário e a versão em includes.chroot"
install -Dm0755 "${root}/reimsctl/target/release/reimsctl" "${inc}/usr/local/bin/reimsctl"
printf '%s\n' "${version}" >"${inc}/usr/lib/reims/version"
chmod 0755 "${inc}"/usr/local/bin/reims-* "${inc}"/usr/local/sbin/reims-* \
	"${inc}/etc/grub.d/01_reims-bootcheck" \
	"${inc}"/etc/kernel/post*.d/zz-reims-update-grub \
	"${img}"/config/hooks/live/*.hook.chroot

if [ "${SKIP_LB:-0}" = "1" ]; then
	echo ">> SKIP_LB=1: staging pronto; pulando o lb build"
	exit 0
fi

echo ">> 3/3 live-build"
cd "${img}"
if [ "$(id -u)" -ne 0 ]; then SUDO="sudo"; else SUDO=""; fi
${SUDO} lb clean --purge || true
./auto/config
${SUDO} lb build

out="${img}/OSX-REIMS-OS-${version}-amd64.iso"
for f in "${img}"/*.hybrid.iso; do
	[ -e "$f" ] && ${SUDO} mv "$f" "$out"
done
echo ">> pronto: ${out}"
ls -lh "${out}" 2>/dev/null && sha256sum "${out}" | tee "${out}.sha256"
