#!/usr/bin/env bash
# OSX-REIMS-OS — provisioner de primeiro boot (Fase 2).
#
# Roda UMA vez após a instalação: detecta a GPU, instala o driver certo, valida
# Vulkan 1.2+ e marca como concluído. NVIDIA é opt-in (driver proprietário).
#
# Idempotente: sai cedo se já provisionado (marca em STAMP).
set -euo pipefail

REIMSCTL="${REIMSCTL:-reimsctl}"
STAMP="${STAMP:-/var/lib/reims/provisioned}"
ASSUME_YES="${ASSUME_YES:-0}"   # 1 = não perguntar (aceita NVIDIA non-free)

log() { printf '\033[1m>> %s\033[0m\n' "$*"; }
err() { printf '\033[31m!! %s\033[0m\n' "$*" >&2; }

require_root() {
  if [ "$(id -u)" -ne 0 ]; then
    err "rode como root"; exit 1
  fi
}

already_done() {
  [ -f "$STAMP" ]
}

install_intel() {
  log "GPU Intel — Mesa ANV (Vulkan)"
  apt-get install -y --no-install-recommends mesa-vulkan-drivers vulkan-tools
}

install_amd() {
  log "GPU AMD — Mesa RADV (Vulkan)"
  apt-get install -y --no-install-recommends mesa-vulkan-drivers vulkan-tools
}

install_nvidia() {
  log "GPU NVIDIA detectada — driver proprietário (non-free)."
  if [ "$ASSUME_YES" != "1" ]; then
    read -rp "Instalar o driver proprietário NVIDIA? [s/N] " ans
    case "${ans:-}" in
      s|S|y|Y) : ;;
      *) err "pulando NVIDIA (opt-in recusado) — sem aceleração Vulkan nesta GPU"; return 0 ;;
    esac
  fi
  # requer os componentes non-free habilitados no apt (feito no build da imagem)
  apt-get install -y nvidia-driver vulkan-tools
}

validate_vulkan() {
  log "validando Vulkan 1.2+"
  if ! command -v vulkaninfo >/dev/null 2>&1; then
    err "vulkaninfo ausente — não foi possível validar Vulkan"
    return 1
  fi
  if vulkaninfo 2>/dev/null | grep -Eq 'apiVersion.*1\.(2|3|[4-9])'; then
    vulkaninfo 2>/dev/null | grep -m1 -i "deviceName" || true
    log "Vulkan OK"
  else
    err "Vulkan 1.2+ não confirmado — a aceleração pode não funcionar"
    return 1
  fi
}

main() {
  require_root
  if already_done; then
    log "já provisionado ($STAMP) — nada a fazer"
    exit 0
  fi

  apt-get update -y

  local vendor
  vendor="$($REIMSCTL gpu-detect 2>/dev/null || echo unknown)"
  log "GPU detectada: $vendor"

  case "$vendor" in
    intel)  install_intel ;;
    amd)    install_amd ;;
    nvidia) install_nvidia ;;
    *)      err "GPU não reconhecida — instalando Mesa genérico"
            apt-get install -y --no-install-recommends mesa-vulkan-drivers vulkan-tools ;;
  esac

  validate_vulkan || err "siga assim mesmo, mas revise a GPU/driver"

  mkdir -p "$(dirname "$STAMP")"
  date -u +%FT%TZ > "$STAMP"
  log "provisionamento concluído"
}

main "$@"
