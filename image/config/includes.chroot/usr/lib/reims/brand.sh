# shellcheck shell=sh
# Nomes, versões e caminhos do appliance, lidos por todos os scripts.
# O projeto vai mudar de nome: a troca começa aqui (e em reimsctl/crates/core/src/brand.rs).

PRODUCT="OSX-REIMS-OS"
VERSION="$(cat /usr/lib/reims/version 2>/dev/null || echo dev)"

# Usuário da sessão (criado pelo instalador, com sudo).
USER_NAME="reims"
DEFAULT_HOSTNAME="reims-os"
BOOTLOADER_ID="reims-os"

STATE_DIR="/var/lib/reims"

# Reims vGPU: baixado da origem e compilado na máquina.
REIMS_VGPU_DIR="/opt/reims-vgpu"
REIMS_VGPU_REPO="https://github.com/steelbrain/reims-vgpu.git"
# Commit fixado por esta versão ("pinned"), o padrão do preparo.
REIMS_VGPU_PINNED="86d492197228df29514a87c3de62b2ce59f2188f"

# Disco do instalador.
MIN_DISK_GB=32
REC_DISK_GB=128
