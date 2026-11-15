//! Real hardware information for the specifications panel.
//!
//! Static data (CPU, GPUs, installed memory, OS) is read once and cached. Only memory usage is re-read, when the
//! UI asks (every few seconds). On Windows: CPU name from the registry, GPUs from DXGI (hardware adapters only, with
//! dedicated video memory), installed memory from `GetPhysicallyInstalledSystemMemory`, OS from the registry with the
//! correct Windows 11 naming. Anything that cannot be read is reported as unavailable, never guessed.

use serde::Serialize;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gpu {
    pub name: String,
    pub dedicated_memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Os {
    pub name: String,
    pub version: Option<String>,
    pub build: Option<String>,
    pub architecture: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticInfo {
    pub cpu: Option<String>,
    pub cpu_cores: Option<usize>,
    pub cpu_threads: Option<usize>,
    pub gpus: Vec<Gpu>,
    /// Physically installed RAM (what the modules add up to).
    pub memory_installed_bytes: Option<u64>,
    /// RAM usable by Windows (installed minus hardware-reserved).
    pub memory_usable_bytes: Option<u64>,
    pub os: Os,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamicInfo {
    pub memory_used_bytes: Option<u64>,
    pub memory_usable_bytes: Option<u64>,
}

static STATIC: OnceLock<StaticInfo> = OnceLock::new();
static SYS: OnceLock<Mutex<sysinfo::System>> = OnceLock::new();

fn sys() -> &'static Mutex<sysinfo::System> {
    SYS.get_or_init(|| Mutex::new(sysinfo::System::new()))
}

pub fn static_info() -> StaticInfo {
    STATIC.get_or_init(read_static).clone()
}

pub fn dynamic_info() -> DynamicInfo {
    let mut s = sys().lock().unwrap();
    s.refresh_memory();
    let total = s.total_memory();
    DynamicInfo {
        memory_used_bytes: (total > 0).then(|| s.used_memory()),
        memory_usable_bytes: (total > 0).then_some(total),
    }
}

fn read_static() -> StaticInfo {
    let mut s = sys().lock().unwrap();
    s.refresh_cpu_all();
    s.refresh_memory();
    let cpus = s.cpus();
    let sys_cpu = cpus.first().map(|c| c.brand().trim().to_string()).filter(|b| !b.is_empty());
    let threads = (!cpus.is_empty()).then_some(cpus.len());
    let cores = sysinfo::System::physical_core_count();
    let usable = s.total_memory();
    drop(s);
    StaticInfo {
        cpu: platform::cpu_name().or(sys_cpu),
        cpu_cores: cores,
        cpu_threads: threads,
        gpus: platform::gpus(),
        memory_installed_bytes: platform::installed_memory().or((usable > 0).then_some(usable)),
        memory_usable_bytes: (usable > 0).then_some(usable),
        os: platform::os(),
    }
}

#[cfg(windows)]
mod platform {
    use super::{Gpu, Os};
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    pub fn cpu_name() -> Option<String> {
        let key = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0").ok()?;
        let name: String = key.get_value("ProcessorNameString").ok()?;
        let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
        (!name.is_empty()).then_some(name)
    }

    pub fn gpus() -> Vec<Gpu> {
        use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE};
        let mut out: Vec<Gpu> = Vec::new();
        unsafe {
            let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() else { return out };
            let mut i = 0;
            while let Ok(adapter) = factory.EnumAdapters1(i) {
                i += 1;
                let Ok(desc) = adapter.GetDesc1() else { continue };
                if (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0 {
                    continue; // Microsoft Basic Render Driver etc.
                }
                let len = desc.Description.iter().position(|&c| c == 0).unwrap_or(desc.Description.len());
                let name = String::from_utf16_lossy(&desc.Description[..len]).trim().to_string();
                if name.is_empty() || out.iter().any(|g| g.name == name) {
                    continue;
                }
                let vram = desc.DedicatedVideoMemory as u64;
                out.push(Gpu { name, dedicated_memory_bytes: (vram > 0).then_some(vram) });
            }
        }
        out
    }

    pub fn installed_memory() -> Option<u64> {
        use windows::Win32::System::SystemInformation::GetPhysicallyInstalledSystemMemory;
        let mut kb: u64 = 0;
        unsafe { GetPhysicallyInstalledSystemMemory(&mut kb).ok()? };
        (kb > 0).then_some(kb * 1024)
    }

    pub fn os() -> Os {
        let key = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion").ok();
        let get = |n: &str| key.as_ref().and_then(|k| k.get_value::<String, _>(n).ok());
        let build = get("CurrentBuildNumber");
        let ubr: Option<u32> = key.as_ref().and_then(|k| k.get_value("UBR").ok());
        let mut name = get("ProductName").unwrap_or_else(|| "Windows".into());
        // ProductName still says "Windows 10" on Windows 11; the build number is authoritative.
        if build.as_deref().and_then(|b| b.parse::<u32>().ok()).map(|b| b >= 22000).unwrap_or(false) {
            name = name.replacen("Windows 10", "Windows 11", 1);
        }
        Os {
            name,
            version: get("DisplayVersion").or_else(|| get("ReleaseId")),
            build: build.map(|b| match ubr {
                Some(u) => format!("{b}.{u}"),
                None => b,
            }),
            architecture: std::env::consts::ARCH.into(),
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{Gpu, Os};

    pub fn cpu_name() -> Option<String> {
        None
    }

    /// Development platforms: read display adapters from `lspci` when present; otherwise report none.
    pub fn gpus() -> Vec<Gpu> {
        let Ok(out) = std::process::Command::new("lspci").output() else { return vec![] };
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.contains("VGA compatible controller") || l.contains("3D controller") || l.contains("Display controller"))
            .filter_map(|l| l.split_once(": ").map(|(_, n)| Gpu { name: n.trim().to_string(), dedicated_memory_bytes: None }))
            .collect()
    }

    pub fn installed_memory() -> Option<u64> {
        None
    }

    pub fn os() -> Os {
        Os {
            name: sysinfo::System::name().unwrap_or_else(|| std::env::consts::OS.into()),
            version: sysinfo::System::os_version(),
            build: sysinfo::System::kernel_version(),
            architecture: std::env::consts::ARCH.into(),
        }
    }
}
