extern crate log;
extern crate winapi;
extern crate winreg;

use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};
use std::{error, ffi::OsStr, fmt, io, iter::once, os::windows::ffi::OsStrExt};
use winapi::{
    shared::ntdef::{HANDLE, NULL},
    um::{
        winbase::{DeregisterEventSource, RegisterEventSourceW, ReportEventW},
        winnt::{EVENTLOG_ERROR_TYPE, EVENTLOG_INFORMATION_TYPE, EVENTLOG_WARNING_TYPE},
    },
};
use winreg::{enums::*, RegKey};

mod eventmsgs;
use eventmsgs::{MSG_DEBUG, MSG_ERROR, MSG_INFO, MSG_TRACE, MSG_WARNING};

const REG_BASEKEY: &str = "SYSTEM\\CurrentControlSet\\Services\\EventLog\\Application";

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    ExePathNotFound,
    RegisterSourceFailed,
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref err) => write!(f, "IO error: {}", err),
            Error::ExePathNotFound => write!(f, "Could not determine executable path"),
            Error::RegisterSourceFailed => write!(f, "Call to RegisterEventSource failed"),
        }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            Error::Io(ref err) => Some(err),
            Error::ExePathNotFound => None,
            Error::RegisterSourceFailed => None,
        }
    }
}

#[derive(Debug)]
pub struct WinLogger {
    handle: HANDLE,
}

unsafe impl Send for WinLogger {}
unsafe impl Sync for WinLogger {}

fn discard_result<R, E>(_result: &Result<R, E>) {
    ()
}

fn win_string(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

pub fn deregister(name: &str) {
    discard_result(&try_deregister(name))
}

pub fn init(name: &str) -> Result<(), SetLoggerError> {
    log::set_boxed_logger(Box::new(WinLogger::new(name)))
        .map(|()| log::set_max_level(LevelFilter::Trace))
}

pub fn register(name: &str) {
    discard_result(&try_register(name))
}

pub fn try_deregister(name: &str) -> Result<(), Error> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let cur_ver = hklm.open_subkey(REG_BASEKEY)?;
    cur_ver.delete_subkey(name).map_err(From::from)
}

pub fn try_register(name: &str) -> Result<(), Error> {
    let current_exe = ::std::env::current_exe()?;
    let exe_path = current_exe.to_str().ok_or(Error::ExePathNotFound)?;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let cur_ver = hklm.open_subkey(REG_BASEKEY)?;
    let app_key = cur_ver.create_subkey(name)?;
    app_key
        .set_value("EventMessageFile", &exe_path)
        .map_err(From::from)
}

impl WinLogger {
    pub fn new(name: &str) -> WinLogger {
        Self::try_new(name).unwrap_or(WinLogger { handle: NULL })
    }

    pub fn try_new(name: &str) -> Result<WinLogger, Error> {
        let wide_name = win_string(name);
        let handle = unsafe { RegisterEventSourceW(std::ptr::null_mut(), wide_name.as_ptr()) };

        if handle == NULL {
            Err(Error::RegisterSourceFailed)
        } else {
            Ok(WinLogger { handle })
        }
    }
}

impl Drop for WinLogger {
    fn drop(&mut self) -> () {
        unsafe { DeregisterEventSource(self.handle) };
    }
}

impl log::Log for WinLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let type_and_msg = match record.level() {
                Level::Error => (EVENTLOG_ERROR_TYPE, MSG_ERROR),
                Level::Warn => (EVENTLOG_WARNING_TYPE, MSG_WARNING),
                Level::Info => (EVENTLOG_INFORMATION_TYPE, MSG_INFO),
                Level::Debug => (EVENTLOG_INFORMATION_TYPE, MSG_DEBUG),
                Level::Trace => (EVENTLOG_INFORMATION_TYPE, MSG_TRACE),
            };

            let msg = win_string(&format!("{:?}", record.args()));
            let mut vec = vec![msg.as_ptr()];

            unsafe {
                ReportEventW(
                    self.handle,
                    type_and_msg.0, // type
                    0,              // category
                    type_and_msg.1, // event id == resource msg id
                    std::ptr::null_mut(),
                    vec.len() as u16,
                    0,
                    vec.as_mut_ptr(),
                    std::ptr::null_mut(),
                )
            };
        }
    }

    fn flush(&self) {}
}
