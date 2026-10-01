# opencore/ — base do OpenCore + injeção de identidade

O caminho x86_64/KVM precisa de um bootloader que forneça SMBIOS/board-id/quirks
ao macOS. Usamos **OpenCore**, com a base vinda do **OSX-KVM** (kholia/OSX-KVM).

## Como funciona

1. **`fetch-base.sh`** — clona o OSX-KVM (versão fixável em `OSX_KVM_REF`) e copia a
   base (`OpenCore.qcow2`) para `opencore/base/` (**gitignored** — não versionamos
   binários de terceiros).
2. **Extração** (host, Fase 2) — o `config.plist` fica dentro da FAT da
   `OpenCore.qcow2`; `oc-image.sh` monta a imagem via **`qemu-nbd`** (do
   `qemu-utils`, sem libguestfs) — requer root + módulo `nbd`.
3. **Injeção** — [`reimsctl-efi::set_platform_identity`](../reimsctl/crates/efi/src/lib.rs)
   grava `PlatformInfo → Generic` (`SystemSerialNumber`, `MLB`, `SystemUUID`, `ROM`)
   com a identidade única da VM (CLI: `reimsctl identity-inject <config.plist>`).
4. **Reempacote** — devolve o `config.plist` à imagem, que vira a `OpenCore.qcow2`
   daquela VM.

A GPU vem do `AppleParavirtGPU.kext` **nativo** do macOS — a base fica enxuta, sem
kext de GPU.

## Licenças

- **OpenCore** (Acidanthera): BSD-2-Clause.
- **OSX-KVM** (kholia): ver a licença do upstream — apenas fazemos *fetch*, não
  redistribuímos os binários aqui.
- Registrar os avisos em `docs/legal/` (a criar) conforme a Fase 3.

## Rails

Uma base por versão de macOS: `macos-13` (Ventura), `macos-14` (Sonoma),
`macos-15` (Sequoia), `macos-26` (Tahoe). Ver [`release.rs`](../reimsctl/crates/core/src/release.rs).
