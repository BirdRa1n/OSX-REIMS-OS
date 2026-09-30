# OSX-REIMS-OS

Um **appliance Debian 13** enxuto que roda **macOS em VM com GPU acelerada** via
[`reims-vgpu`](https://github.com/steelbrain/reims-vgpu). Instalador simples (disco +
senha, estilo Debian); depois de instalado, um gerenciador próprio (`reimsctl`) baixa
o macOS direto da Apple, cria/roda VMs aceleradas e atualiza o sistema.

> ⚠️ **Experimental / educacional.** Não substitui um Mac. Rodar macOS fora de
> hardware Apple viola a EULA da Apple — uso por sua conta e risco. Nenhuma imagem do
> macOS é distribuída por este projeto; ela é baixada dos servidores da Apple na sua
> máquina.

## Como funciona (resumo)

1. **Boot pela ISO (USB)** → instalador mínimo: escolhe disco + define senha → grava e reinicia.
2. **Primeiro boot**: auto-detecta GPU, instala driver (Intel/AMD via Mesa; NVIDIA opt-in),
   valida Vulkan 1.2+, prepara o `reims-vgpu`.
3. **Operação** (`reimsctl`): baixar macOS, criar/rodar VM acelerada, atualizar, snapshots.

## Camadas

| Camada | O quê | Origem |
|---|---|---|
| `reims-vgpu` | GPU virtual (decode → Vulkan) | reuso (LGPL-3.0) |
| `reimsctl` | gerenciador de VMs (QEMU/KVM, OpenCore, serials, download) | **nosso**, Rust |
| ISO | distribuição Debian 13 appliance | **nossa**, live-build |

## Documentação

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — desenho completo & decisões.
- [docs/ROADMAP.md](docs/ROADMAP.md) — fases e checklist.

## Requisitos (host)

x86_64, UEFI, VT-x/AMD-V, GPU com **Vulkan 1.2+** (Intel / AMD / NVIDIA), disco
dedicado (≥ 32 GB; 128 GB+ recomendado), Ethernet com DHCP.

## Estado

Fase 0 — fundações. Ver o roadmap.
