#[allow(unused_imports)]
use anyhow::{anyhow, Context, Result};

#[cfg(windows)]
use windows::Win32::Security::Credentials::{
    CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_FLAGS,
    CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
};
#[cfg(windows)]
use windows::core::PWSTR;

const CREDENTIAL_PREFIX: &str = "shelldeck/";

/// Save a password to Windows Credential Manager.
#[cfg(windows)]
pub fn save_password(tunnel_id: &str, password: &str) -> Result<()> {
    let target = format!("{}{}", CREDENTIAL_PREFIX, tunnel_id);
    let target_wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
    let password_bytes = password.as_bytes();

    let mut cred = CREDENTIALW {
        Flags: CRED_FLAGS(0),
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(target_wide.as_ptr() as *mut u16),
        CredentialBlobSize: password_bytes.len() as u32,
        CredentialBlob: password_bytes.as_ptr() as *mut u8,
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        ..Default::default()
    };

    unsafe {
        CredWriteW(&mut cred, 0).context("Failed to write credential to Windows Credential Manager")?;
    }
    Ok(())
}

/// Load a password from Windows Credential Manager.
#[cfg(windows)]
pub fn load_password(tunnel_id: &str) -> Result<Option<String>> {
    let target = format!("{}{}", CREDENTIAL_PREFIX, tunnel_id);
    let target_wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();

    let mut cred_ptr: *mut CREDENTIALW = std::ptr::null_mut();

    let result = unsafe {
        CredReadW(
            windows::core::PCWSTR(target_wide.as_ptr()),
            CRED_TYPE_GENERIC,
            0,
            &mut cred_ptr,
        )
    };

    match result {
        Ok(()) => {
            let password = unsafe {
                let cred = &*cred_ptr;
                let blob = std::slice::from_raw_parts(
                    cred.CredentialBlob,
                    cred.CredentialBlobSize as usize,
                );
                let pwd = String::from_utf8_lossy(blob).to_string();
                CredFree(cred_ptr as *const std::ffi::c_void);
                pwd
            };
            Ok(Some(password))
        }
        Err(_) => Ok(None),
    }
}

/// Delete a password from Windows Credential Manager.
#[cfg(windows)]
pub fn delete_password(tunnel_id: &str) -> Result<()> {
    let target = format!("{}{}", CREDENTIAL_PREFIX, tunnel_id);
    let target_wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let _ = CredDeleteW(
            windows::core::PCWSTR(target_wide.as_ptr()),
            CRED_TYPE_GENERIC,
            0,
        );
    }
    Ok(())
}

// Stubs for non-Windows (won't be used but allows compilation)
#[cfg(not(windows))]
pub fn save_password(_tunnel_id: &str, _password: &str) -> Result<()> {
    Err(anyhow!("Credential Manager is only available on Windows"))
}

#[cfg(not(windows))]
pub fn load_password(_tunnel_id: &str) -> Result<Option<String>> {
    Err(anyhow!("Credential Manager is only available on Windows"))
}

#[cfg(not(windows))]
pub fn delete_password(_tunnel_id: &str) -> Result<()> {
    Err(anyhow!("Credential Manager is only available on Windows"))
}
