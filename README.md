# OSX-REIMS-OS

Um **sistema dedicado, baseado no Debian 13**, que transforma um computador num
appliance de **VMs macOS com GPU acelerada** pelo
[Reims vGPU](https://github.com/steelbrain/reims-vgpu). Uma ISO inicializável:
instala num disco, detecta a GPU, instala os drivers de vídeo sozinho e liga direto no
console das VMs. Sem desktop, sem configuração manual.

> ⚠️ **Experimental, alfa, para estudos e testes.** Não substitui um Mac. Rodar macOS
> fora de hardware Apple contraria a licença da Apple — por sua conta e risco. Nada da
> Apple é distribuído aqui: o macOS é baixado dos servidores da Apple na sua máquina.

## Do pendrive ao macOS

1. **Instalar** (sem internet) — menu de boot com *Install*, *Install (safe graphics)* e
   *Advanced / Rescue*. O instalador de texto pergunta o **disco** (aviso abaixo de
   128 GB, recusa abaixo de 32 GB), a confirmação digitando **`ERASE`**, o **nome do
   computador** e a **senha** do usuário `reims` (com sudo). Grava, pede para tirar o
   pendrive e reinicia.
2. **Primeiro setup** (cabo de rede, DHCP) — confere a rede, examina CPU,
   virtualização, KVM, IOMMU e cada GPU, mostra o **plano de drivers** do Debian e só
   instala com confirmação (NVIDIA só com autorização; CUDA nunca). Reinicia uma vez,
   sozinho, se o driver precisa ler o firmware, e valida os gráficos de verdade:
   **READY / SOFTWARE / FAIL**.
3. **Preparar** — o console do gerenciador abre sozinho em tela cheia, mostra a licença e
   prepara o ambiente em **10 etapas** (grupos, KVM, Rust, código e compilação do
   Reims vGPU num commit fixado).
4. **Usar** — *Create new VM*: número 10–99, instalador (Recovery da Apple, Offline ou
   imagem completa), versão (Ventura, Sonoma, Sequoia, Tahoe) e disco. A VM abre em tela
   cheia; uma VM pode **iniciar sozinha** quando o computador liga.

Ao sair do console: `Enter` volta, `s` abre um terminal, `r` reinicia, `p` desliga.
`Ctrl+Alt+F2` dá um login em texto; o SSH já vem ligado (`ssh reims@<ip>`).

## Comandos

| Comando | O que faz |
|---|---|
| `reimsctl` | o console (criar, importar, gerenciar, logs, manutenção) |
| `reimsctl version` | versões do sistema, kernel, reimsctl, reims-ctl e Reims vGPU |
| `reimsctl hw detect\|plan\|validate\|report` | hardware, plano de drivers, validação dos gráficos |
| `reimsctl diagnostics [--save] [--splash]` | relatório completo, sem números de série |
| `reimsctl prepare [--update --latest\|--rebuild]` | prepara/atualiza/recompila o Reims vGPU |
| `sudo reims-firstboot --force` | refaz o primeiro setup (ex.: trocou a GPU) |

## Camadas

| Camada | O quê | Origem |
|---|---|---|
| ISO + instalador + primeiro setup + sessão | Debian 13 mínimo, `cage` (Wayland kiosk) | **este repo** (`image/`) |
| `reimsctl` | console, hardware, diagnóstico | **este repo** (Rust, `reimsctl/`) |
| `reims-ctl` + QEMU + GPU virtual | cria e roda as VMs (OpenCore, identidade, download da Apple) | [Reims vGPU](https://github.com/steelbrain/reims-vgpu), compilado na máquina |

## Requisitos

x86_64 com **UEFI** (sem BIOS legado), Intel VT-x ou AMD-V, GPU Intel/AMD/NVIDIA com
**Vulkan 1.2+**, um **disco inteiro** (32 GB mínimo, 128 GB+ recomendado; é apagado),
**Ethernet com DHCP** (Wi-Fi não é suportado), pendrive de ~1 GB.

## Documentação

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — desenho e decisões.
- [docs/ROADMAP.md](docs/ROADMAP.md) — estado e próximos passos.
- [image/README.md](image/README.md) — como gerar a ISO.
