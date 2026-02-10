use std::io;
use std::path::Path;

#[cfg(unix)]
pub fn lock_down_dir(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o700);
    std::fs::set_permissions(path, perms)
}

#[cfg(unix)]
pub fn lock_down_file(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(path, perms)
}

#[cfg(windows)]
pub fn lock_down_dir(path: &Path) -> io::Result<()> {
    set_owner_only_dacl(path, true)
}

#[cfg(windows)]
pub fn lock_down_file(path: &Path) -> io::Result<()> {
    set_owner_only_dacl(path, false)
}

#[cfg(windows)]
fn set_owner_only_dacl(path: &Path, is_dir: bool) -> io::Result<()> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_INSUFFICIENT_BUFFER, GetLastError, HANDLE, LocalFree,
    };
    use windows_sys::Win32::Security::Authorization::{
        EXPLICIT_ACCESS_W, NO_MULTIPLE_TRUSTEE, SE_FILE_OBJECT, SET_ACCESS, SetEntriesInAclW,
        SetNamedSecurityInfoW, TRUSTEE_IS_SID, TRUSTEE_IS_USER, TRUSTEE_W,
    };
    use windows_sys::Win32::Security::{
        ACL, DACL_SECURITY_INFORMATION, GetTokenInformation, PROTECTED_DACL_SECURITY_INFORMATION,
        TOKEN_QUERY, TOKEN_USER, TokenUser,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    const GENERIC_ALL: u32 = 0x1000_0000;
    const SUB_CONTAINERS_AND_OBJECTS_INHERIT: u32 = 0x00000003;

    let mut token: HANDLE = 0;
    let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
    if opened == 0 {
        return Err(io::Error::last_os_error());
    }

    let mut needed = 0u32;
    unsafe {
        GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &mut needed);
    }
    if unsafe { GetLastError() } != ERROR_INSUFFICIENT_BUFFER {
        unsafe {
            CloseHandle(token);
        }
        return Err(io::Error::last_os_error());
    }

    let mut buffer = vec![0u8; needed as usize];
    let info_ok = unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
        )
    };
    unsafe {
        CloseHandle(token);
    }
    if info_ok == 0 {
        return Err(io::Error::last_os_error());
    }

    let token_user = unsafe { &*(buffer.as_ptr() as *const TOKEN_USER) };
    let user_sid = token_user.User.Sid;

    let trustee = TRUSTEE_W {
        pMultipleTrustee: ptr::null_mut(),
        MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
        TrusteeForm: TRUSTEE_IS_SID,
        TrusteeType: TRUSTEE_IS_USER,
        ptstrName: user_sid as *mut u16,
    };

    let explicit_access = EXPLICIT_ACCESS_W {
        grfAccessPermissions: GENERIC_ALL,
        grfAccessMode: SET_ACCESS,
        grfInheritance: if is_dir {
            SUB_CONTAINERS_AND_OBJECTS_INHERIT
        } else {
            0
        },
        Trustee: trustee,
    };

    let mut new_acl: *mut ACL = ptr::null_mut();
    let acl_result =
        unsafe { SetEntriesInAclW(1, &explicit_access, ptr::null_mut(), &mut new_acl) };
    if acl_result != 0 {
        return Err(io::Error::from_raw_os_error(acl_result as i32));
    }

    let wide: Vec<u16> = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let sec_result = unsafe {
        SetNamedSecurityInfoW(
            wide.as_ptr() as *mut u16,
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            new_acl,
            ptr::null_mut(),
        )
    };

    if !new_acl.is_null() {
        unsafe {
            LocalFree(new_acl as *mut _);
        }
    }

    if sec_result != 0 {
        return Err(io::Error::from_raw_os_error(sec_result as i32));
    }

    Ok(())
}
