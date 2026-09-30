# provisioner/ — primeiro boot (Fase 2)

Scripts que rodam **uma vez**, após a instalação no disco:

- detecção de GPU (Intel/AMD/NVIDIA) e instalação do driver correto;
- NVIDIA = **opt-in** (driver proprietário non-free, requer autorização explícita);
- validação de **Vulkan 1.2+** (`vulkaninfo`);
- deploy do `reims-vgpu` pré-compilado (+ rebuild como fallback);
- rede (DHCP), NTP, e entrega ao `reimsctl`.

Ver [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) §4.C.
