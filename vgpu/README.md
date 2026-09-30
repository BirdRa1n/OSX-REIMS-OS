# vgpu/ — integração do reims-vgpu

Pin de versão e build reproduzível do
[`reims-vgpu`](https://github.com/steelbrain/reims-vgpu) (LGPL-3.0), usado como
device de GPU do QEMU (`reims-vgpu-pci`, backend Vulkan no host x86_64/KVM).

- adicionar como git submodule (versão fixada) — **não** editar o upstream aqui;
- `build.sh` alinhado ao QEMU que a imagem empacota;
- respeitar a LGPL (ver [../docs/legal](../docs/legal) — a criar).

Ver [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) §4.G e §9.
