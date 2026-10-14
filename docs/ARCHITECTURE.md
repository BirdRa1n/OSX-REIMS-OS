# OSX-REIMS-OS — Arquitetura

> Documento vivo. Desenho de um **appliance Debian 13 bootável** que roda macOS em VM
> com GPU acelerada pelo Reims vGPU. Atualizado em 2026-10-14 (v0.2.0).

## 1. O que é

Uma ISO que transforma um PC x86_64 num computador que só existe para rodar VMs macOS:
instala num disco inteiro, prepara o hardware e liga direto no console das VMs.

**Não-objetivos:** substituir um Mac; dual boot; Apple Silicon; distribuir qualquer coisa
da Apple; atualização no lugar (uma versão nova é reinstalada pelo pendrive).

## 2. Quem faz o quê

| Camada | Papel |
|---|---|
| **Imagem** (`image/`) | live-build Debian 13; instalador; primeiro setup; sessão kiosk; tela de carregamento; GRUB |
| **`reimsctl`** (`reimsctl/`) | console; `hw` (detecção, plano, validação); diagnóstico; ponte para o `reims-ctl` |
| **Reims vGPU** (terceiros) | GPU virtual (Metal → Vulkan), o **QEMU dele** e o **`reims-ctl`**, que cria as VMs (download do macOS, imagem OpenCore com identidade própria, OVMF, disco) e as roda |

Decisão central: **não remontar o que o upstream já faz.** O QEMU do Debian não tem o
device do Reims vGPU; o upstream traz um QEMU próprio e uma ferramenta de VMs madura. O
appliance compila o upstream num commit fixado e o `reimsctl` é a camada de UX por cima.

## 3. Fluxo e componentes

```
pendrive (GRUB UEFI, tema próprio)
 ├─ Install / Install (safe graphics)  → reims.mode=install
 └─ Advanced / Rescue: rescue shell · live · check medium · firmware setup
        │
        ▼  sistema live (boot=live), tty1 com login automático de root
   reims-getty → profile.d → reims-live → reims-installer (whiptail, offline)
        disco → ERASE → hostname → senha → GPT (ESP 512M + ext4) → unsquashfs
        → fstab, usuário reims (sudo,kvm,render,video), SSH, initramfs, GRUB UEFI
        │
        ▼  sistema instalado: GRUB oculto (Esc mostra; volta sozinho após boot falho)
   Plymouth "Starting …" → reims-hwscan (hardware a cada boot)
   reims-firstboot.service (tty1, 1ª vez): rede → hw → plano → drivers → build deps
        → reboot único se preciso → validação READY/SOFTWARE/FAIL
        │
        ▼  getty@tty1 com login automático do usuário reims
   reims-session: cage (Wayland kiosk) + foot em tela cheia → reimsctl console
        licença → reims-prepare (10 etapas) → menu principal
        Create / Import / Manage / Logs / Maintenance → reims-ctl
        janelas das VMs: mesmo compositor, tela cheia (borderless, resolução do host)
        sair → menu da sessão (Enter · s terminal · r reboot · p desliga)
```

### 3.1 Instalador (`usr/local/sbin/reims-installer`)
Não usa o debian-installer (`--debian-installer none`): roda no próprio live e copia o
squashfs, então instala **sem internet**. Remove os pacotes live do alvo, zera o
machine-id e as chaves SSH, cria o usuário, trava o root, instala o GRUB (entrada própria
+ cópia `--removable` para firmwares que perdem a NVRAM) e grava `reims_boot_ok=1`.

### 3.2 Primeiro setup (`usr/local/sbin/reims-firstboot`)
Etapas com estado em `/var/lib/reims/firstboot.stage` para continuar depois do reboot.
O plano vem de `reimsctl hw plan`; NVIDIA usa `nvidia-detect` para escolher o pacote.
IOMMU é opcional (`/etc/default/grub.d/reims-iommu.cfg`). Grava o veredito em
`/var/lib/reims/graphics`; FAIL bloqueia o preparo no console.

### 3.3 Validação dos gráficos (`reimsctl hw validate`)
Lê `vulkaninfo --summary` e casa cada device Vulkan com a GPU PCI (vendor:device).
READY só se a **GPU do monitor** (`boot_vga`) responde com Vulkan ≥ 1.2 num device que
não é CPU; só llvmpipe → SOFTWARE; o resto → FAIL.

### 3.4 Preparo (`usr/local/bin/reims-prepare`)
Com o usuário da sessão; sudo só quando preciso. 1 sistema · 2 GPU/Vulkan · 3 grupos ·
4 KVM (`/dev/kvm`, `vm.max_map_count=2097152`, `udmabuf` em AMD) · 5 Rust (rustup) ·
6 código do Reims vGPU (`/opt/reims-vgpu`, commit **pinned** ou **latest**) · 7 deps de
build · 8 build (`scripts/install-ctl.sh` do upstream) · 9 `reims-ctl doctor` · 10 pronto
(`~/.local/state/reims-os/prepared.json`). Build falho num commit novo volta ao anterior,
cuja compilação continua instalada (o upstream só troca `qemu/current` no sucesso).

### 3.5 Console (`reimsctl`, crate `tui`)
Tela de tecla única. VMs `macos-<10..99>` do `reims-ctl`, com estados *Ready to install*,
*Ready for first use*, *Installed*, *Running*, *Faulty*. Primeira inicialização pergunta
memória e CPUs e mostra os passos de instalação (Disk Utility, APFS e o
`METAL_ERROR_MODE=5` que o upstream exige). Configurações por VM via `reims-ctl set`.
Início automático: uma VM instalada, iniciada no primeiro console da sessão. Um log por
ação em `~/.local/state/reims-os/logs/` (100 mantidos).

## 4. Disco e caminhos

| Caminho | Conteúdo |
|---|---|
| `/var/lib/reims/` | `installed`, `firstboot.done`, `graphics`, `hw.json` |
| `/opt/reims-vgpu/` | checkout do Reims vGPU |
| `~reims/.local/share/reims-ctl/` | QEMU compilado, VMs (`vms/macos-NN`), instaladores baixados |
| `~reims/.local/state/reims-os/` | licença, `prepared.json`, metadados das VMs, autostart, logs |
| `/usr/lib/reims/brand.sh`, `reimsctl/crates/core/src/brand.rs` | nomes e caminhos (troca de nome) |

## 5. Licenças

- Este repositório: GPL-3.0-or-later.
- Reims vGPU: GPL-2.0-or-later OR LGPL-3.0-or-later — baixado e compilado na máquina do
  usuário; nenhum binário dele é redistribuído na ISO.
- macOS: sempre direto da Apple, pelo `reims-ctl`/`reims-util` do upstream.

## 6. Decisões

| Decisão | Por quê |
|---|---|
| Motor de VM = upstream `reims-ctl` | o QEMU do Debian não tem o device; o upstream mantém OpenCore, OVMF, identidade e download |
| Instalador próprio no live | offline, só 4 perguntas, GPT fixo; o d-i pergunta demais |
| Drivers fora da ISO | a ISO fica pequena; cada máquina recebe só o que precisa, do Debian |
| `cage` + `foot` | sessão kiosk sem desktop; janelas das VMs no mesmo compositor |
| `grub-efi-amd64-bin` + hooks de kernel | o pacote `grub-efi-amd64` tentaria instalar o GRUB no chroot do build |
| Kernel dos backports | GPUs recentes |
