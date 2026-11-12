use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

#[allow(dead_code)]
pub const PAM_SUCCESS: c_int = 0;
#[allow(dead_code)]
pub const PAM_PROMPT_ECHO_OFF: c_int = 1;
#[allow(dead_code)]
pub const PAM_PROMPT_ECHO_ON: c_int = 2;
#[allow(dead_code)]
pub const PAM_ERROR_MSG: c_int = 3;
#[allow(dead_code)]
pub const PAM_TEXT_INFO: c_int = 4;

#[repr(C)]
pub struct PamMessage {
    pub msg_style: c_int,
    pub msg: *const c_char,
}

#[repr(C)]
pub struct PamResponse {
    pub resp: *mut c_char,
    pub resp_retcode: c_int,
}

#[repr(C)]
pub struct PamConv {
    pub conv: Option<
        unsafe extern "C" fn(
            c_int,
            *mut *const PamMessage,
            *mut *mut PamResponse,
            *mut c_void,
        ) -> c_int,
    >,
    pub appdata_ptr: *mut c_void,
}

enum PamHandle {}

type PamStartFn = unsafe extern "C" fn(
    *const c_char,
    *const c_char,
    *const PamConv,
    *mut *mut PamHandle,
) -> c_int;

type PamAuthenticateFn = unsafe extern "C" fn(*mut PamHandle, c_int) -> c_int;
type PamEndFn = unsafe extern "C" fn(*mut PamHandle, c_int) -> c_int;
type PamStrerrorFn = unsafe extern "C" fn(*mut PamHandle, c_int) -> *const c_char;

pub struct PamClient {
    pam_start: PamStartFn,
    pam_authenticate: PamAuthenticateFn,
    pam_end: PamEndFn,
    pam_strerror: PamStrerrorFn,
}

impl PamClient {
    pub fn new() -> Result<Self, String> {
        unsafe {
            let lib_name = CString::new("libpam.so.0").map_err(|e| e.to_string())?;
            let handle = libc::dlopen(lib_name.as_ptr(), libc::RTLD_NOW);
            if handle.is_null() {
                // Fallback to libpam.so
                let lib_name2 = CString::new("libpam.so").map_err(|e| e.to_string())?;
                let handle2 = libc::dlopen(lib_name2.as_ptr(), libc::RTLD_NOW);
                if handle2.is_null() {
                    return Err("Gagal memuat libpam.so.0: pustaka PAM tidak ditemukan".to_string());
                }
                return Self::from_handle(handle2);
            }
            Self::from_handle(handle)
        }
    }

    unsafe fn from_handle(handle: *mut c_void) -> Result<Self, String> {
        let load = |name: &str| -> Result<*mut c_void, String> {
            let cname = CString::new(name).map_err(|e| e.to_string())?;
            let sym = libc::dlsym(handle, cname.as_ptr());
            if sym.is_null() {
                Err(format!("Simbol PAM {} tidak ditemukan", name))
            } else {
                Ok(sym)
            }
        };

        let pam_start: PamStartFn = std::mem::transmute(load("pam_start")?);
        let pam_authenticate: PamAuthenticateFn = std::mem::transmute(load("pam_authenticate")?);
        let pam_end: PamEndFn = std::mem::transmute(load("pam_end")?);
        let pam_strerror: PamStrerrorFn = std::mem::transmute(load("pam_strerror")?);

        Ok(Self {
            pam_start,
            pam_authenticate,
            pam_end,
            pam_strerror,
        })
    }

    pub fn verify_password(&self, service: &str, user: &str, password: &str) -> Result<(), String> {
        let service_c = CString::new(service).map_err(|e| e.to_string())?;
        let user_c = CString::new(user).map_err(|e| e.to_string())?;
        let pass_c = CString::new(password).map_err(|e| e.to_string())?;

        unsafe extern "C" fn conv_callback(
            num_msg: c_int,
            msg: *mut *const PamMessage,
            resp: *mut *mut PamResponse,
            appdata_ptr: *mut c_void,
        ) -> c_int {
            if num_msg <= 0 || msg.is_null() || resp.is_null() {
                return 19; // PAM_CONV_ERR
            }

            let responses = libc::calloc(num_msg as usize, std::mem::size_of::<PamResponse>())
                as *mut PamResponse;
            if responses.is_null() {
                return 19;
            }

            let pass_ptr = appdata_ptr as *const c_char;

            for i in 0..num_msg as usize {
                let m = *msg.add(i);
                if m.is_null() {
                    continue;
                }
                let style = (*m).msg_style;
                if style == PAM_PROMPT_ECHO_OFF || style == PAM_PROMPT_ECHO_ON {
                    let r = libc::strdup(pass_ptr);
                    (*responses.add(i)).resp = r;
                    (*responses.add(i)).resp_retcode = 0;
                } else {
                    (*responses.add(i)).resp = ptr::null_mut();
                    (*responses.add(i)).resp_retcode = 0;
                }
            }

            *resp = responses;
            PAM_SUCCESS
        }

        let conv = PamConv {
            conv: Some(conv_callback),
            appdata_ptr: pass_c.as_ptr() as *mut c_void,
        };

        unsafe {
            let mut pamh: *mut PamHandle = ptr::null_mut();
            let res = (self.pam_start)(
                service_c.as_ptr(),
                user_c.as_ptr(),
                &conv,
                &mut pamh,
            );

            if res != PAM_SUCCESS {
                return Err(format!("pam_start gagal dengan kode {}", res));
            }

            let auth_res = (self.pam_authenticate)(pamh, 0);
            let err_msg = if auth_res != PAM_SUCCESS {
                let err_ptr = (self.pam_strerror)(pamh, auth_res);
                if !err_ptr.is_null() {
                    Some(CStr::from_ptr(err_ptr).to_string_lossy().into_owned())
                } else {
                    Some(format!("Kode error PAM: {}", auth_res))
                }
            } else {
                None
            };

            (self.pam_end)(pamh, auth_res);

            if let Some(err) = err_msg {
                Err(err)
            } else {
                Ok(())
            }
        }
    }
}
