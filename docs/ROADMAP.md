# OSX-REIMS-OS — Roadmap

Deriva de [ARCHITECTURE.md](ARCHITECTURE.md). Marque `[x]` conforme concluir.

## Fase 0 — Fundações & PoC (sem imagem)
Provar que o stack roda **na mão** antes de investir em ISO.

- [ ] Ambiente Debian 13 x86_64 com KVM habilitado (`kvm-ok`).
- [ ] Compilar `reims-vgpu` upstream (Rust + deps QEMU) — pin de versão em `vgpu/`.
- [ ] Baixar macOS Recovery direto da Apple (`reimsctl/crates/macos`).
- [ ] Subir VM macOS via QEMU/KVM + OVMF + OpenCore + `-device reims-vgpu-pci`.
- [ ] Confirmar **GPU acelerada** no guest (não framebuffer básico).
- [ ] Congelar a linha do QEMU que funciona em `docs/QEMU-LINE.md`.

## Fase 1 — `reimsctl` mínimo (Rust)
- [ ] `core`: modelo de VM + assembler da linha do QEMU (esqueleto pronto).
- [ ] `efi`: geração de identidade única (serial/MLB/UUID/ROM/MAC) + injeção OpenCore.
- [x] `macos`: download do Recovery da Apple (protocolo osrecovery; CLI `fetch-macos`).
- [ ] `core`: criar/iniciar/parar/listar VM (storage qcow2 + snapshots).
- [ ] `tui`: menu de console (ratatui).
- [ ] `cli`: binário `reimsctl` amarrando tudo.

## Fase 2 — Imagem/ISO (live-build)
- [x] `image/`: live-build Debian 13, sem GUI, console do `reimsctl` no tty1.
- [x] Instalador Fase 1 (disco + senha) — preseed (a validar em Debian real).
- [x] `provisioner/`: detecção de GPU, drivers (Intel/AMD Mesa; NVIDIA opt-in),
      validação Vulkan 1.2+ (deploy do `reims-vgpu` pré-compilado: hook no build-iso).
- [ ] Validar o build da ISO numa Debian 13 (bash -n só cobre sintaxe).
- [x] CI (`.github/workflows/iso.yml`) que gera a ISO (workflow_dispatch + tags v*).

## Fase 3 — Atualizações & polimento
- [ ] `updater`: repo APT próprio assinado + rebuild do reims-vgpu.
- [ ] Snapshots/rails, monitor de VM, múltiplas VMs.
- [ ] Testes em hardware real Intel / AMD / NVIDIA.
- [ ] Docs de usuário + página do produto.
