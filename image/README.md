# image/ — build da ISO (Debian 13 + live-build)

Configuração do `live-build` que gera `osx-reims-os-<versão>.iso`: sistema base
Debian 13 sem GUI, autologin do `reimsctl` no tty1, pacotes de QEMU/OVMF/Mesa/
firmware e o `reims-vgpu` **pré-compilado** embutido.

- `config/`   — saída de `lb config` (gitignored; gerada por script).
- `hooks/`    — hooks de build (instalar reimsctl, habilitar serviços, limpar).
- `preseed/`  — preseed do instalador (Fase 1: disco + senha).

Fase 2 do roadmap. Ver [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) §4.A.
