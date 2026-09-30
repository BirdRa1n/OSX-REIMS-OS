//! reimsctl-core — modelo de domínio do gerenciador de VMs.
//!
//! Contém os tipos centrais (GPU, versão de macOS, configuração de VM) e o
//! *assembler* da linha de comando do QEMU. É a fundação sobre a qual `efi`,
//! `macos`, `tui` e o binário `cli` se apoiam.

pub mod gpu;
pub mod qemu;
pub mod release;
pub mod vm;

pub use gpu::GpuVendor;
pub use release::MacosRelease;
pub use vm::VmConfig;
