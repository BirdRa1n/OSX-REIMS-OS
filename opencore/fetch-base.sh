#!/usr/bin/env bash
# Busca a base do OpenCore a partir do OSX-KVM (kholia/OSX-KVM).
#
# NÃO versionamos binários de terceiros neste repo: este script clona o OSX-KVM
# numa versão fixada e copia a base do OpenCore para `opencore/base/` (gitignored).
# A partir dela, a crate `reimsctl-efi` injeta a identidade única por VM.
#
# ⚠️ Fixe OSX_KVM_REF num commit específico após validar na Fase 0
# (ver docs/QEMU-LINE.md). O default `master` é só para o primeiro PoC.
#
# Uso: opencore/fetch-base.sh
set -euo pipefail

OSX_KVM_REPO="${OSX_KVM_REPO:-https://github.com/kholia/OSX-KVM}"
OSX_KVM_REF="${OSX_KVM_REF:-master}"   # TODO(fase-0): fixar um commit

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cache="${here}/.cache/OSX-KVM"
dest="${here}/base"

mkdir -p "$(dirname "${cache}")" "${dest}"

if [ ! -d "${cache}/.git" ]; then
  echo ">> clonando ${OSX_KVM_REPO} (${OSX_KVM_REF})"
  git clone "${OSX_KVM_REPO}" "${cache}"
fi
git -C "${cache}" fetch --all --tags --quiet
git -C "${cache}" checkout --quiet "${OSX_KVM_REF}"

# A base do OpenCore no OSX-KVM é a imagem EFI pré-montada (FAT dentro de qcow2).
src_img="${cache}/OpenCore/OpenCore.qcow2"
if [ ! -f "${src_img}" ]; then
  echo "!! não achei ${src_img} — o layout do OSX-KVM mudou; ajuste este script" >&2
  exit 1
fi

cp -v "${src_img}" "${dest}/OpenCore.qcow2"
echo ">> base copiada para ${dest}/OpenCore.qcow2"
echo ">> commit da base:"
git -C "${cache}" rev-parse HEAD

cat <<'NOTE'

Próximos passos (host, Fase 2):
  1. extrair EFI/OC/config.plist de dentro da OpenCore.qcow2
     (libguestfs `guestfish`/`guestmount` ou `mtools` mcopy sobre a partição FAT);
  2. rodar `reimsctl identity-inject <config.plist>` (crate efi);
  3. reempacotar o config.plist na imagem e usar como OpenCore.qcow2 da VM.
NOTE
