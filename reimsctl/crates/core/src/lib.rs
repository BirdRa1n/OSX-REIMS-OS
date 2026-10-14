//! reimsctl-core — o núcleo do gerenciador do appliance.
//!
//! - [`hw`]: detecção do hardware, plano de drivers e validação dos gráficos
//!   (o mesmo código do first setup);
//! - [`ctl`]: ponte para o `reims-ctl` do Reims vGPU, que cria e roda as VMs;
//! - [`state`]: licença, preparo, metadados das VMs, início automático e logs;
//! - [`diag`]: o relatório de diagnóstico;
//! - [`brand`]: nomes e caminhos (o projeto vai mudar de nome).

pub mod brand;
pub mod ctl;
pub mod diag;
pub mod hw;
pub mod release;
pub mod state;
pub mod sys;

pub use hw::{GpuVendor, Verdict};
pub use release::MacosRelease;
