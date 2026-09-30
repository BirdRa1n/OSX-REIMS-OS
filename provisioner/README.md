# provisioner/ — primeiro boot (Fase 2)

Scripts que rodam **uma vez**, após a instalação no disco.

## `provision.sh`

Orquestrador idempotente de first-boot:

1. detecta a GPU via `reimsctl gpu-detect` (Intel/AMD/NVIDIA/unknown — lê o sysfs PCI);
2. instala o driver/ICD Vulkan correto:
   - **Intel** → `mesa-vulkan-drivers` (ANV)
   - **AMD** → `mesa-vulkan-drivers` (RADV)
   - **NVIDIA** → `nvidia-driver` (proprietário, **opt-in**; `ASSUME_YES=1` aceita)
3. valida **Vulkan 1.2+** (`vulkaninfo`);
4. marca conclusão em `STAMP` (`/var/lib/reims/provisioned`) — sai cedo se já feito.

Variáveis: `REIMSCTL`, `STAMP`, `ASSUME_YES`.

> Os componentes `non-free` do apt (para o driver NVIDIA) são habilitados no build
> da imagem (`image/`), não aqui. O deploy do `reims-vgpu` pré-compilado também é
> tarefa do build da imagem.

Ligado ao boot por um serviço systemd (a criar em `image/`), antes de entregar o
console ao `reimsctl`. Ver [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) §4.C.
