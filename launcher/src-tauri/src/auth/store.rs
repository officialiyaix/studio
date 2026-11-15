//! Credential storage.
//!
//! Windows: the account record is encrypted with DPAPI (`CryptProtectData`, current-user scope, Stabled-specific
//! entropy) and written to `%APPDATA%\Stabled\secure\account.bin`. Only the same Windows user on the same machine
//! can decrypt it. Other platforms (development builds only) keep the session in memory and never write it to disk.

use crate::error::AppResult;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[cfg(windows)]
const ENTROPY: &[u8] = b"Stabled launcher account v1";

fn path() -> std::path::PathBuf {
    crate::paths::secure_dir().join("account.bin")
}

pub fn is_persistent() -> bool {
    cfg!(windows)
}

#[cfg(windows)]
mod dpapi {
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 }
    }

    unsafe fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(out.pbData as *mut core::ffi::c_void)));
        v
    }

    pub fn protect(data: &[u8], entropy: &[u8]) -> Option<Vec<u8>> {
        let input = blob(data);
        let ent = blob(entropy);
        let mut out = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptProtectData(&input, None, Some(&ent), None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out).ok()?;
            Some(take(out))
        }
    }

    pub fn unprotect(data: &[u8], entropy: &[u8]) -> Option<Vec<u8>> {
        let input = blob(data);
        let ent = blob(entropy);
        let mut out = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptUnprotectData(&input, None, Some(&ent), None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out).ok()?;
            Some(take(out))
        }
    }
}

#[cfg(windows)]
pub fn save<T: Serialize>(value: &T) -> AppResult<()> {
    let plain = serde_json::to_vec(value)?;
    let sealed = dpapi::protect(&plain, ENTROPY)
        .ok_or_else(|| crate::error::AppError::new("AUTH_STORE", "Windows could not protect your sign-in. You'll need to sign in again next time."))?;
    crate::util::fsx::write_atomic(&path(), &sealed)
}

#[cfg(windows)]
pub fn load<T: DeserializeOwned>() -> Option<T> {
    let sealed = std::fs::read(path()).ok()?;
    let plain = dpapi::unprotect(&sealed, ENTROPY)?;
    serde_json::from_slice(&plain).ok()
}

#[cfg(not(windows))]
static MEMORY: std::sync::Mutex<Option<Vec<u8>>> = std::sync::Mutex::new(None);

#[cfg(not(windows))]
pub fn save<T: Serialize>(value: &T) -> AppResult<()> {
    *MEMORY.lock().unwrap() = Some(serde_json::to_vec(value)?);
    Ok(())
}

#[cfg(not(windows))]
pub fn load<T: DeserializeOwned>() -> Option<T> {
    MEMORY.lock().unwrap().as_ref().and_then(|b| serde_json::from_slice(b).ok())
}

pub fn clear() {
    let _ = std::fs::remove_file(path());
    #[cfg(not(windows))]
    {
        *MEMORY.lock().unwrap() = None;
    }
}
