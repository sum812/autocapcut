/// fingerprint.rs — Thu thập Machine Fingerprint từ các thành phần phần cứng.
///
/// Chỉ hoạt động trên Windows. Trên non-Windows trả fingerprint giả để dev.

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FingerprintComponent {
    pub kind: String,  // "MachineGuid" | "WindowsProductId" | "CpuId" | "MotherboardSerial" | "DiskSerial"
    pub value: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MachineFingerprint {
    pub id: String,                         // SHA256 của tất cả components
    pub components: Vec<FingerprintComponent>,
    pub component_count: usize,
}

#[cfg(target_os = "windows")]
mod platform {
    use super::FingerprintComponent;
    use sha2::{Digest, Sha256};

    /// Đọc giá trị từ Windows Registry (HKLM).
    fn read_registry(subkey: &str, value: &str) -> Option<String> {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let key = hklm.open_subkey(subkey).ok()?;
        let val: String = key.get_value(value).ok()?;
        let trimmed = val.trim().to_string();
        if trimmed.is_empty() || trimmed == "None" || trimmed == "To be filled by O.E.M." {
            None
        } else {
            Some(trimmed)
        }
    }

    /// Lấy CPU ID thông qua CPUID instruction.
    fn get_cpu_id() -> Option<String> {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use std::arch::x86_64::__cpuid;
            let result = __cpuid(1);
            // EAX chứa Family/Model/Stepping
            Some(format!("{:08X}{:08X}", result.eax, result.ecx))
        }
        #[cfg(not(target_arch = "x86_64"))]
        None
    }

    /// Lấy giá trị từ WMI qua PowerShell (không cần COM/WMI crate).
    fn get_wmi_value(class: &str, property: &str) -> Option<String> {
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!(
                    "(Get-WmiObject -Class {} | Select-Object -First 1 -ExpandProperty {} 2>$null)",
                    class, property
                ),
            ])
            .output()
            .ok()?;

        let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if val.is_empty()
            || val == "None"
            || val.contains("To be filled")
            || val.contains("Default string")
            || val.contains("Not Specified")
        {
            None
        } else {
            Some(val)
        }
    }

    pub fn generate() -> super::MachineFingerprint {
        let mut components: Vec<FingerprintComponent> = Vec::new();

        // Component 1: MachineGuid (stable nhất)
        if let Some(v) = read_registry(r"SOFTWARE\Microsoft\Cryptography", "MachineGuid") {
            components.push(FingerprintComponent { kind: "MachineGuid".into(), value: v });
        }

        // Component 2: Windows Product ID
        if let Some(v) = read_registry(
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
            "ProductId",
        ) {
            components.push(FingerprintComponent { kind: "WindowsProductId".into(), value: v });
        }

        // Component 3: CPU ID
        if let Some(v) = get_cpu_id() {
            components.push(FingerprintComponent { kind: "CpuId".into(), value: v });
        }

        // Component 4: Motherboard Serial (WMI)
        if let Some(v) = get_wmi_value("Win32_BaseBoard", "SerialNumber") {
            components.push(FingerprintComponent { kind: "MotherboardSerial".into(), value: v });
        }

        // Component 5: Disk Serial (drive Windows, thường C:)
        if let Some(v) = get_wmi_value("Win32_DiskDrive", "SerialNumber") {
            components.push(FingerprintComponent { kind: "DiskSerial".into(), value: v });
        }

        // Hash: sort theo kind → join → SHA256
        let mut sorted = components.clone();
        sorted.sort_by(|a, b| a.kind.cmp(&b.kind));
        let raw = sorted
            .iter()
            .map(|c| format!("{}:{}", c.kind, c.value))
            .collect::<Vec<_>>()
            .join("|");

        let id = format!("{:x}", Sha256::digest(raw.as_bytes()));
        let component_count = components.len();
        super::MachineFingerprint { id, components, component_count }
    }
}

/// Tạo machine fingerprint.
pub fn generate_fingerprint() -> MachineFingerprint {
    #[cfg(target_os = "windows")]
    return platform::generate();

    // Non-Windows: dev/test fingerprint
    #[cfg(not(target_os = "windows"))]
    {
        MachineFingerprint {
            id: "dev-fingerprint-non-windows".into(),
            components: vec![FingerprintComponent {
                kind: "DevMode".into(),
                value: "non-windows".into(),
            }],
            component_count: 1,
        }
    }
}
