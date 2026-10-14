//! `reimsctl diagnostics`: tudo o que é preciso para entender um problema, num
//! texto que pode ser compartilhado — sem senhas, chaves nem números de série.

use crate::{brand, ctl, hw, state, sys};
use std::fmt::Write;
use std::fs;

/// Linhas com estas palavras são escondidas (identidade das VMs e do host).
const REDACT: &[&str] = &[
    "serial", "mlb", "uuid", "rom ", "rom=", "password", "passwd", "token", "secret",
];

pub fn redact(text: &str) -> String {
    text.lines()
        .map(|l| {
            let low = l.to_ascii_lowercase();
            if REDACT.iter().any(|w| low.contains(w)) {
                match l.split_once([':', '=']) {
                    Some((k, _)) => format!("{k}: [hidden]"),
                    None => "[hidden]".into(),
                }
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn section(out: &mut String, title: &str, body: &str) {
    let _ = writeln!(out, "\n## {title}\n");
    out.push_str(body.trim_end());
    out.push('\n');
}

fn file(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| format!("({path}: not present)\n"))
}

fn sh(cmd: &str) -> String {
    sys::capture("sh", &["-c", cmd])
}

/// O relatório completo.
pub fn report() -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# {} diagnostics · {} · {}",
        brand::PRODUCT,
        brand::VERSION,
        state::now_stamp()
    );

    section(&mut out, "Version", &version_text());
    section(
        &mut out,
        "System",
        &(file("/etc/os-release") + &sh("uname -a")),
    );

    let h = hw::detect();
    let p = hw::plan(&h);
    let v = hw::validate(&h);
    section(&mut out, "Hardware and graphics", &hw::report(&h, &p, &v));
    section(
        &mut out,
        "Saved state",
        &format!(
            "first setup: {}\ngraphics: {}\nfirst setup log tail:\n{}",
            if brand::system_state_dir().join("firstboot.done").exists() {
                "done"
            } else {
                "not done"
            },
            hw::saved_verdict().map(|x| x.as_str()).unwrap_or("none"),
            sh("tail -n 40 /var/log/reims-firstboot.log 2>/dev/null || echo '(no log)'")
        ),
    );
    section(
        &mut out,
        "PCI (display)",
        &sh("lspci -nnk | grep -A3 -Ei 'vga|3d|display'"),
    );
    section(
        &mut out,
        "DRM",
        &sh("ls -l /dev/dri 2>&1; ls /sys/class/drm 2>&1"),
    );
    section(
        &mut out,
        "Kernel modules",
        &sh("lsmod | grep -Ei 'i915|xe|amdgpu|nvidia|kvm|udmabuf' || true"),
    );
    section(
        &mut out,
        "Vulkan",
        &sys::capture("vulkaninfo", &["--summary"]),
    );
    section(
        &mut out,
        "KVM and IOMMU",
        &sh(
            "ls -l /dev/kvm /dev/udmabuf 2>&1; ls /sys/class/iommu 2>&1; \
             echo max_map_count=$(cat /proc/sys/vm/max_map_count); cat /proc/cmdline",
        ),
    );
    section(
        &mut out,
        "Network",
        &sh("ip -br link; ip -br addr; ip route"),
    );
    section(
        &mut out,
        "Storage",
        &sh("lsblk -o NAME,SIZE,TYPE,FSTYPE,MOUNTPOINTS; df -h /"),
    );
    section(
        &mut out,
        "Manager",
        &format!(
            "license accepted: {}\nprepared: {:?}\nautostart: {}\nreims-ctl: {}\n{}",
            state::license_accepted(),
            state::prepared(),
            state::autostart().unwrap_or_else(|| "none".into()),
            ctl::version().unwrap_or_else(|| "not installed".into()),
            if ctl::installed() {
                sys::capture(&ctl::bin().to_string_lossy(), &["list"])
                    + &sys::capture(&ctl::bin().to_string_lossy(), &["doctor"])
            } else {
                String::new()
            }
        ),
    );
    section(
        &mut out,
        "Boot",
        &sh(
            "systemd-analyze 2>&1; systemd-analyze blame 2>&1 | head -n 25; \
             systemd-analyze critical-chain 2>&1 | head -n 25",
        ),
    );
    section(
        &mut out,
        "Services",
        &sh(
            "systemctl --failed --no-pager 2>&1; \
             systemctl status --no-pager reims-firstboot reims-hwscan getty@tty1 2>&1 | head -n 60",
        ),
    );
    section(
        &mut out,
        "Kernel errors",
        &sh("journalctl -k -b -p err --no-pager 2>&1 | tail -n 60"),
    );
    section(
        &mut out,
        "Previous boot",
        &sh("journalctl -b -1 -p warning --no-pager 2>&1 | tail -n 60"),
    );
    section(&mut out, "Splash", &splash_text());
    redact(&out)
}

/// `reimsctl version`.
pub fn version_text() -> String {
    let os = fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("PRETTY_NAME="))
                .map(|v| v.trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "unknown".into());
    let kernel = sys::output("uname", &["-r"]).unwrap_or_default();
    let shipped = fs::read_to_string("/usr/lib/reims/version")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "-".into());
    let prepared = state::prepared()
        .map(|p| format!("{} ({}, prepared {})", short(&p.commit), p.follow, p.at))
        .unwrap_or_else(|| "not prepared".into());
    format!(
        "{} {}\nimage    {shipped}\nsystem   {os}\nkernel   {}\nreimsctl {}\nreims-ctl {}\nReims vGPU {prepared}\n",
        brand::PRODUCT,
        brand::VERSION,
        kernel.trim(),
        brand::VERSION,
        ctl::version().unwrap_or_else(|| "not installed".into()),
    )
}

fn short(c: &str) -> &str {
    &c[..c.len().min(8)]
}

/// `reimsctl diagnostics --splash`: por que a tela de carregamento não apareceu.
pub fn splash_text() -> String {
    let cmdline = fs::read_to_string("/proc/cmdline").unwrap_or_default();
    let mut s = String::new();
    let mut check = |ok: bool, good: &str, bad: &str| {
        let _ = writeln!(
            s,
            "{} {}",
            if ok { "✓" } else { "✗" },
            if ok { good } else { bad }
        );
    };
    check(
        sys::has("plymouth"),
        "plymouth is installed",
        "plymouth is not installed",
    );
    check(
        cmdline.split_whitespace().any(|w| w == "splash"),
        "the kernel command line asks for the splash",
        "no `splash` on the kernel command line (/etc/default/grub)",
    );
    check(
        !cmdline.contains("nomodeset"),
        "kernel mode setting is on",
        "nomodeset is on: there is no graphics mode for the splash",
    );
    let theme = sys::output("plymouth-set-default-theme", &[]).unwrap_or_default();
    check(
        theme.trim() == "reims",
        "the splash theme is reims",
        &format!("the splash theme is `{}`, not reims", theme.trim()),
    );
    let initrd = sh("lsinitramfs /boot/initrd.img-$(uname -r) 2>/dev/null | grep -c plymouth");
    check(
        initrd.trim().parse::<u32>().unwrap_or(0) > 0,
        "the initramfs has plymouth",
        "the initramfs has no plymouth: sudo update-initramfs -u",
    );
    let drm = fs::read_dir("/sys/class/drm")
        .map(|d| {
            d.flatten()
                .any(|e| e.file_name().to_string_lossy().starts_with("card"))
        })
        .unwrap_or(false);
    check(
        drm,
        "a DRM device exists",
        "no DRM device: the GPU driver did not load early enough",
    );
    s.push_str(&sh(
        "journalctl -b --no-pager -u plymouth-start -u plymouth-quit 2>&1 | tail -n 15",
    ));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_identity_lines() {
        let t = "state running\nSystemSerialNumber: C02XYZ\nMLB = ABC\nmemory 8G";
        let r = redact(t);
        assert!(!r.contains("C02XYZ"));
        assert!(!r.contains("ABC"));
        assert!(r.contains("memory 8G"));
        assert!(r.contains("SystemSerialNumber: [hidden]"));
    }
}
