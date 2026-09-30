# OSX-REIMS-OS — Arquitetura & Plano do Projeto

> Documento vivo. Desenho de como vamos construir um **appliance Debian bootável**
> que roda macOS em VM com aceleração gráfica via **reims-vgpu**, com instalador
> simples (disco + senha) e, pós-instalação, um gerenciador próprio de VMs +
> atualizações.
>
> Status: **desenho inicial** (2026-09-30). Decisões travadas no rodapé.

---

## 1. O que estamos construindo

Uma **imagem de sistema operacional (ISO)** enxuta, baseada em **Debian 13**, que
transforma um PC x86_64 em um *appliance* dedicado a rodar **macOS virtualizado com
GPU acelerada**. O usuário:

1. Dá boot pela ISO (USB), roda um **instalador simples** (escolhe disco + define
   senha, estilo Debian) e o sistema é gravado no disco.
2. No **primeiro boot pós-instalação**, o sistema auto-detecta hardware, instala
   drivers de GPU e compila/prepara o `reims-vgpu`.
3. A partir daí, cai num **console/gerenciador de VMs** (TUI própria) onde pode:
   baixar imagens do macOS direto dos servidores da Apple, criar/rodar VMs, e
   **instalar atualizações do programa**.

É o mesmo *nicho* do **vGPU OS**, mas construído do zero como produto próprio,
reusando **apenas** o `reims-vgpu` (open-source, LGPL-3.0). O gerenciador de VMs
(equivalente ao OSX-REIMS, que é fechado/pago) é **nosso**.

### Não-objetivos (explícitos)
- Não substitui um Mac real; foco em teste/estudo/experimentação.
- Não é dual-boot: o appliance toma o disco inteiro (como o vGPU OS).
- Apple Silicon/HVF **fora do escopo do appliance** (uma ISO Debian x86_64 não roda
  em Mac ARM; o `reims-vgpu` tem esse caminho, mas não por aqui).
- Não distribuímos imagens do macOS — elas são baixadas da Apple pelo usuário.

---

## 2. As três camadas do "stack REIMS" (referências)

| Camada | Projeto de referência | Licença | O que faremos |
|---|---|---|---|
| **GPU virtual** | `reims-vgpu` (steelbrain) — Rust + shims QEMU + Metal2Vulkan; decodifica o command stream do `AppleParavirtGPU.kext` no host e executa via Vulkan | **LGPL-3.0** | **Reusar** (compilar como dependência de terceiros) |
| **Gerenciador de VMs** | OSX-REIMS (Universo Hackintosh) — TUI, QEMU/KVM, EFI OpenCore, serials, download macOS | Fechado/pago | **Construir o nosso** do zero |
| **Distribuição/imagem** | vGPU OS — ISO Debian 13 appliance, sem GUI, boot direto no console | Fechado/pago | **Construir a nossa** ISO |

Chave técnica do `reims-vgpu`: o macOS **já traz** o driver `AppleParavirtGPU.kext`.
Expondo um device de GPU paravirtualizada que esse kext reconhece, não é preciso
driver custom no guest. O host decodifica os comandos e executa no **Vulkan** da
GPU real (Intel ANV / AMD RADV / NVIDIA). É isso que dá aceleração sem "Hackintosh
de GPU".

---

## 3. Arquitetura em camadas

```
┌───────────────────────────────────────────────────────────────┐
│  HARDWARE  (x86_64, UEFI, VT-x/AMD-V, GPU Vulkan 1.2+)          │
├───────────────────────────────────────────────────────────────┤
│  OSX-REIMS-OS  (Debian 13, sem desktop)                         │
│    • kernel + KVM  • drivers GPU (i915/xe, amdgpu, nvidia)      │
│    • Mesa (ANV/RADV) / Vulkan ICD                               │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │  reims-vgpu (host)  — decode → Vulkan                    │  │
│  ├─────────────────────────────────────────────────────────┤  │
│  │  QEMU/KVM  + OVMF  +  device reims-vgpu-pci              │  │
│  │  ┌───────────────────────────────────────────────────┐  │  │
│  │  │  Guest macOS  (OpenCore → AppleParavirtGPU.kext)   │  │  │
│  │  └───────────────────────────────────────────────────┘  │  │
│  └─────────────────────────────────────────────────────────┘  │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │  reimsctl  (NOSSO gerenciador — TUI + core)             │  │
│  │   installer │ provisioner │ vm-manager │ efi-gen │       │  │
│  │   macos-fetch │ updater                                  │  │
│  └─────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────┘
```

---

## 4. Componentes que vamos construir

### A. Build da imagem (`image/`) — **live-build**
- **Debian 13 (trixie)** via `live-build` (`lb config`/`lb build`).
- Sem ambiente gráfico. Console em full-screen (getty → `reimsctl` autologin no tty1).
- Pacotes base: `qemu-system-x86`, `qemu-utils`, `ovmf`, `python3`, drivers Mesa,
  firmware (`firmware-linux`, `firmware-misc-nonfree`), `linux-image-amd64`, rede.
- Runtime do `reimsctl` (ver decisão de linguagem no §6).
- **Instalador**: usaremos o **debian-installer** em modo *preseed parcial* OU um
  instalador próprio minimalista sobre a live (a decidir no §6) — o requisito é
  "escolher disco + senha", nada além.
- Saída: `osx-reims-os-<versão>.iso` (alvo: leve, poucas centenas de MB).

### B. Instalador — Fase 1 (disco + senha)
Fluxo mínimo, estilo Debian:
1. Detecta discos, o usuário **escolhe o disco alvo** (aviso de apagamento total).
2. Define **senha** (usuário `admin`/root padrão Debian).
3. Particiona (GPT: ESP + root ext4/`btrfs`), instala o sistema base no disco,
   grava o bootloader (GRUB UEFI), reinicia.
4. **Nada de configuração de VM/macOS aqui** — isso é pós-instalação.

### C. Provisioner — Fase 2 (primeiro boot)
Roda uma vez, após instalado:
- Auto-detecta GPU (Intel/AMD/NVIDIA) e instala o driver certo:
  - Intel: `i915`/`xe` + Mesa **ANV**
  - AMD: `amdgpu` + Mesa **RADV**
  - **NVIDIA**: driver **proprietário** (com passo de autorização explícita do
    usuário — non-free) + verificação Vulkan
- Valida **Vulkan 1.2+** (`vulkaninfo`), reporta se a GPU não serve.
- Compila/instala o **reims-vgpu** (toolchain Rust + deps do QEMU) — ou usa binário
  pré-compilado embutido na imagem, se optarmos por isso (§6, trade-off tamanho×offline).
- Configura rede (DHCP/Ethernet), NTP, e sai para o `reimsctl` (console principal).

### D. `reimsctl` — Gerenciador de VMs (NOSSO, o coração)
TUI em tela cheia + um "core" chamável por script. Funções:
- **VMs**: criar, listar, iniciar, parar, editar (RAM/CPU/disco escolhidos no launch),
  status/monitor.
- **Storage**: cria disco qcow2 por VM; snapshots copy-on-write ("rails", inspirado
  no reims-vgpu) para boots reproduzíveis/não-destrutivos.
- **Boot da VM**: monta a linha do QEMU (KVM, OVMF, `-device reims-vgpu-pci`, CPU
  host-passthrough com flags macOS, `isa-applesmc`, `SMBIOS`, etc.) e injeta o EFI.
- Integra **E/F/G** abaixo.

### E. Gerador de EFI / OpenCore + identidade (`efi-gen`)
- Empacota um **OpenCore** base (config.plist parametrizado) por versão de macOS
  (Ventura 13 → Sonoma 14 → Sequoia 15 → Tahoe 26).
- **Gera identidade única por VM**: Serial, MLB, UUID (SmUUID), ROM, MAC — para não
  colidir entre instâncias (mesmo espírito do OSX-REIMS "400+ customizações").
- Aplica quirks/kexts mínimos necessários ao guest em KVM (Lilu/VirtualSMC etc. só
  se necessário; a GPU vem do `AppleParavirtGPU.kext` nativo).

### F. Download do macOS (`macos-fetch`)
- Baixa **direto dos servidores da Apple** (mesma abordagem do `fetch-macOS`/gibMacOS
  do ecossistema OSX-KVM): pega a **imagem de Recovery** da versão escolhida.
- Monta a mídia de instalação e liga ao fluxo de criação de VM (boot no Recovery →
  instala macOS no disco qcow2).
- Nenhuma imagem da Apple é redistribuída por nós.

### G. Integração `reims-vgpu` (`vgpu/`)
- Pin de versão (submodule/vendored) do `reims-vgpu` upstream.
- Script de build reproduzível (Rust + deps QEMU) alinhado ao QEMU que empacotamos.
- Caminho x86_64/KVM/Vulkan (device `reims-vgpu-pci`, backend Vulkan via metal2vulkan).
- Flags úteis expostas no `reimsctl` (ex.: `REIMS_VGPU_DMABUF`, `REIMS_VGPU_DRAW_LOG`).

### H. Updater (`updater`)
- **Só disponível pós-instalação** (requisito seu). Atualiza:
  - o `reimsctl` e componentes do OSX-REIMS-OS (nosso repo/apt próprio ou releases GH),
  - o `reims-vgpu` (rebuild da versão nova),
  - bases de EFI/OpenCore.
- Mecanismo candidato: **repositório APT próprio** (assinado) + `unattended`/comando
  no menu, ou releases no GitHub verificadas por assinatura. (Decisão no §6.)

---

## 5. Jornada do usuário

```
[USB boot]
   │
   ▼
Fase 1 — INSTALADOR (simples, estilo Debian)
   • escolhe disco  • define senha  • grava sistema  → reboot
   │
   ▼
Fase 2 — PRIMEIRO BOOT (provisioner, automático)
   • detecta GPU + instala driver (NVIDIA pede autorização)
   • valida Vulkan  • prepara/compila reims-vgpu  • rede
   │
   ▼
OPERAÇÃO — reimsctl (console principal)
   • Baixar macOS (Apple)   • Criar/rodar VM (GPU acelerada)
   • Atualizações           • Snapshots / gerência
```

Note a separação que você pediu: **instalação é mínima**; **todas as opções**
(atualizações, download de macOS, criar VM) só aparecem **depois** de instalado.

---

## 6. Decisões técnicas em aberto (a resolver antes/durante a Fase 0)

1. **Linguagem do `reimsctl`**: ✅ **TRAVADO: Rust** (casa com reims-vgpu, binário
   único, TUI com `ratatui`).
2. **Instalador**: debian-installer + preseed (robusto, feio) vs instalador próprio
   TUI sobre a live (bonito, mais trabalho). → *Começar com preseed, evoluir p/ TUI.*
3. **reims-vgpu na imagem**: ✅ **TRAVADO: pré-compilado embutido** (offline, boot
   rápido) + fallback de rebuild no updater.
4. **Canal de updates**: repo APT próprio assinado vs GitHub Releases. → *APT próprio.*
5. **Base de OpenCore**: manter nossa própria árvore config.plist por versão macOS.
6. **Nome/marca do produto**: hoje usamos `OSX-REIMS-OS` (nome do repo) como working
   name.

---

## 7. Estrutura de repositório proposta

```
OSX-REIMS-OS/
  docs/                 arquitetura, roadmap, oncore, macos-fetch, gpu, legal
  image/                config do live-build (Debian 13) + hooks + package lists
    config/
    hooks/
    preseed/            (se instalador via debian-installer)
  installer/            instalador Fase 1 (disco + senha) — se for próprio
  provisioner/          scripts do 1º boot (detecção GPU, drivers, vulkan, reims-vgpu)
  reimsctl/             NOSSO gerenciador (core + TUI)
    core/               vm-manager, storage/snapshots, qemu-launcher
    efi-gen/            OpenCore + geração de serials/MLB/UUID/ROM/MAC
    macos-fetch/        download do macOS (Apple)
    updater/            atualizações pós-instalação
    tui/                interface de console
  vgpu/                 pin/submodule do reims-vgpu + build scripts
  opencore/             árvore base de OpenCore por versão de macOS
  packaging/            .deb do reimsctl, metadados do repo APT
  scripts/              build-iso.sh, dev helpers
  .github/workflows/    CI: build da ISO + build do reims-vgpu + releases
```

---

## 8. Roadmap por fases

**Fase 0 — Fundações & prova de conceito (sem imagem ainda)**
- [ ] Numa Debian 13 x86_64 comum: compilar `reims-vgpu`, subir uma VM macOS via
      QEMU/KVM + OpenCore + `reims-vgpu-pci` **na mão** e confirmar GPU acelerada.
- [ ] Documentar a linha de QEMU exata que funciona (`docs/QEMU-LINE.md`).
- [ ] Validar download do macOS (Recovery) e instalação end-to-end.
- *Meta: provar que o stack roda antes de investir na imagem.*

**Fase 1 — `reimsctl` mínimo (o gerenciador)**
- [ ] Core: criar/rodar/parar VM a partir de config declarativa.
- [ ] `efi-gen`: gerar identidade única + injetar OpenCore.
- [ ] `macos-fetch`: baixar Recovery da Apple pelo menu.
- [ ] TUI básica.

**Fase 2 — Imagem/ISO (live-build)**
- [ ] `image/` com live-build Debian 13, autologin no `reimsctl`.
- [ ] Instalador Fase 1 (disco + senha) + provisioner Fase 2 (drivers/GPU/Vulkan).
- [ ] Suporte NVIDIA (driver proprietário + autorização).
- [ ] CI que gera a ISO.

**Fase 3 — Atualizações & polimento**
- [ ] Updater (repo APT próprio assinado).
- [ ] Snapshots/rails, monitor de VM, múltiplas VMs.
- [ ] Documentação de usuário + testes em hardware Intel/AMD/NVIDIA.

---

## 9. Riscos & pontos legais

- **Legal/EULA Apple**: rodar macOS fora de hardware Apple viola a EULA da Apple.
  Posicionar como educacional/experimental (como as referências fazem). **Nunca**
  redistribuir imagens do macOS — só baixar da Apple na máquina do usuário.
- **LGPL-3.0 do reims-vgpu**: podemos usar/linkar, mas respeitar a LGPL (avisos,
  possibilidade de relink, disponibilizar modificações do próprio reims-vgpu se
  alteradas). Nosso código pode ter licença própria; documentar em `docs/legal`.
- **NVIDIA proprietário**: non-free, requer autorização do usuário (não pode ir na
  ISO principal como default silencioso). Modelar como opt-in no provisioner.
- **Estabilidade**: `reims-vgpu` é **experimental** (alpha). Esperar bugs de render.
- **Manutenção do OpenCore**: cada nova versão de macOS pode quebrar quirks — custo
  recorrente.

---

## 10. Próximos passos imediatos

1. **Resolver §6.1 e §6.3** (linguagem do reimsctl + pré-compilar vs compilar).
2. **Fase 0**: montar o ambiente de PoC numa Debian 13 e provar o stack na mão.
3. Criar `docs/QEMU-LINE.md` e `docs/ROADMAP.md` a partir deste doc.
4. Inicializar o esqueleto de diretórios do §7.

---

### Decisões travadas (2026-09-30)
- Gerenciador de VMs: **construir o nosso** (reusar só o `reims-vgpu` LGPL).
- Entregável primário: **ISO Debian bootável (appliance)**.
- Base: **Debian 13 + live-build**.
- GPU: Intel/AMD (Mesa ANV/RADV) **+ NVIDIA proprietário** (opt-in).
- Host: **x86_64 / KVM** (Apple Silicon fora do appliance).
- `reimsctl`: **Rust** (workspace Cargo, TUI `ratatui`).
- `reims-vgpu` na imagem: **pré-compilado embutido** + rebuild via updater.
