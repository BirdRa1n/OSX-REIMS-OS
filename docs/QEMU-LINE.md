# QEMU-LINE — PoC da Fase 0 (provar o stack na mão)

Objetivo: numa **Debian 13 x86_64** com KVM, subir **uma VM macOS acelerada** por
`reims-vgpu` **manualmente**, e **congelar a linha do QEMU que funciona**. Só depois
disso vale a pena investir na ISO (Fase 2).

> ⚠️ Educacional/experimental. Rodar macOS fora de hardware Apple viola a EULA da
> Apple. Nenhuma imagem do macOS é distribuída aqui — baixe da Apple na sua máquina.
> A chave **OSK do AppleSMC** é propriedade da Apple; forneça a sua.

Esta linha é a referência viva do assembler em
[`reimsctl/crates/core/src/qemu.rs`](../reimsctl/crates/core/src/qemu.rs). Ao ajustar
uma, ajuste a outra.

---

## 0. Pré-requisitos do host

```bash
# virtualização
egrep -c '(vmx|svm)' /proc/cpuinfo          # > 0
sudo apt install -y cpu-checker && sudo kvm-ok

# pacotes base
sudo apt install -y \
  qemu-system-x86 qemu-utils ovmf \
  git build-essential pkg-config \
  mesa-vulkan-drivers vulkan-tools libvulkan-dev \
  python3 python3-venv

# toolchain Rust (para compilar o reims-vgpu)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Vulkan disponível? (precisa 1.2+)
vulkaninfo | grep -i "apiVersion\|deviceName"
```

- Intel → `mesa-vulkan-drivers` (ANV). AMD → idem (RADV).
- **NVIDIA** → driver **proprietário** (`nvidia-driver`, non-free) — opt-in.
- Adicione seu usuário aos grupos: `sudo usermod -aG kvm,libvirt $USER` (relogar).

---

## 1. Compilar o reims-vgpu

> ⚠️ Os passos exatos vêm do **README do upstream**
> (<https://github.com/steelbrain/reims-vgpu>) — confirme lá. Esqueleto:

```bash
mkdir -p ~/reims && cd ~/reims
git clone https://github.com/steelbrain/reims-vgpu
cd reims-vgpu
# seguir o build do upstream (Rust + shims/QEMU vendored + metal2vulkan)
# caminho alvo desta PoC: host x86_64 / KVM / Vulkan, device `reims-vgpu-pci`
```

TODO(fase-0): registrar aqui o comando de build definitivo e o binário/plugin QEMU
resultante + como o QEMU o carrega (device `reims-vgpu-pci`).

---

## 2. OpenCore + firmware

```bash
mkdir -p ~/reims/vm && cd ~/reims/vm
# OVMF (firmware): copie o par CODE (ro) + VARS (gravável por-VM)
cp /usr/share/OVMF/OVMF_CODE.fd .
cp /usr/share/OVMF/OVMF_VARS.fd ./OVMF_VARS.fd
```

Para o OpenCore, a base do ecossistema OSX-KVM funciona bem para PoC
(<https://github.com/kholia/OSX-KVM> → `OpenCore.qcow2`). Na árvore final ele virá de
[`opencore/`](../opencore/README.md) com identidade única injetada pela crate `efi`.

---

## 3. Baixar o macOS (Recovery, direto da Apple)

```bash
# abordagem fetch-macOS / gibMacOS (ecossistema OSX-KVM)
git clone https://github.com/kholia/OSX-KVM ~/reims/OSX-KVM
cd ~/reims/OSX-KVM
python3 fetch-macOS-v2.py        # escolher a versão (Ventura/Sonoma/Sequoia/Tahoe)
# gera BaseSystem.dmg → converter para BaseSystem.img
qemu-img convert -f dmg -O raw BaseSystem.dmg ~/reims/vm/BaseSystem.img
```

Na árvore final isso é a crate [`reimsctl-macos`](../reimsctl/crates/macos/src/lib.rs).

---

## 4. Criar o disco da VM

```bash
cd ~/reims/vm
qemu-img create -f qcow2 disk.qcow2 128G
```

---

## 5. Linha do QEMU (a validar)

Espelha o `qemu::build_args`. Ajustar até bootar com GPU acelerada:

```bash
OSK="<sua-chave-OSK-do-AppleSMC>"

qemu-system-x86_64 \
  -enable-kvm \
  -machine q35 \
  -m 12288 \
  -smp cores=6,sockets=1 \
  -cpu host,vendor=GenuineIntel,+invtsc,+hypervisor,kvm=on,vmware-cpuid-freq=on \
  -device isa-applesmc,osk="$OSK" \
  -drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.fd \
  -drive if=pflash,format=raw,file=OVMF_VARS.fd \
  -device ich9-ahci,id=sata \
  -device ide-hd,bus=sata.2,drive=OpenCoreBoot \
    -drive id=OpenCoreBoot,if=none,format=qcow2,file=OpenCore.qcow2 \
  -device ide-hd,bus=sata.3,drive=BaseSystem \
    -drive id=BaseSystem,if=none,format=raw,file=BaseSystem.img \
  -device ide-hd,bus=sata.4,drive=MacHDD \
    -drive id=MacHDD,if=none,format=qcow2,file=disk.qcow2 \
  -device reims-vgpu-pci \
  -usb -device usb-tablet -device usb-kbd \
  -netdev user,id=net0 -device vmxnet3,netdev=net0 \
  -smbios type=2
```

TODO(fase-0): confirmar props exatas do `-device reims-vgpu-pci` (e variáveis de
ambiente `REIMS_VGPU_*`) conforme o upstream; ajustar flags de `-cpu` por CPU.

---

## 6. Instalar e verificar aceleração

1. Boote → menu do OpenCore → **Recovery/instalador**.
2. Disk Utility: apague o `MacHDD` (APFS) → instale o macOS nele.
3. Após instalar, remova o `BaseSystem` da linha e boote do `MacHDD`.
4. **Verificar GPU acelerada** (não framebuffer básico):
   - "Sobre este Mac" mostra a GPU virtual / aceleração;
   - animações fluidas, `AppleParavirtGPU.kext` carregado
     (`kextstat | grep -i paravirt`);
   - sem tearing/lentidão de framebuffer puro.

---

## 7. Congelar o resultado

Quando bootar acelerado de forma estável, registre **exatamente**:

- [ ] a linha do QEMU final (colar em `## Linha congelada` abaixo);
- [ ] versão do `reims-vgpu` (commit) e como foi compilado;
- [ ] versão do macOS e do OpenCore;
- [ ] GPU/driver do host testado (Intel/AMD/NVIDIA);
- [ ] qualquer `REIMS_VGPU_*` necessário.

Depois: alinhar `reimsctl/crates/core/src/qemu.rs` a esta linha e seguir para a Fase 1.

---

## Linha congelada

_(preencher após o PoC)_
