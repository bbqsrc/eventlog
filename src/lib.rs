mod eventmsgs;

use std::convert::TryInto;
use std::ffi::OsStr;
use std::io;
use std::iter::once;
use std::{os::windows::ffi::OsStrExt, ptr::null_mut};

use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};
use registry::{Data, Hive, Security};
use winapi::{
    shared::ntdef::HANDLE,
    um::{
        winbase::{DeregisterEventSource, RegisterEventSourceW, ReportEventW},
        winnt::{EVENTLOG_ERROR_TYPE, EVENTLOG_INFORMATION_TYPE, EVENTLOG_WARNING_TYPE},
    },
};

use crate::eventmsgs::{MSG_DEBUG, MSG_ERROR, MSG_INFO, MSG_TRACE, MSG_WARNING};

const REG_BASEKEY: &str = r"SYSTEM\CurrentControlSet\Services\EventLog\Application";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not determine executable path")]
    ExePathNotFound,

    #[error("Call to RegisterEventSource failed")]
    RegisterSourceFailed(#[from] io::Error),

    #[error("Failed to modify registry key")]
    RegKey(#[from] registry::key::Error),

    #[error("Failed to modify registry value")]
    RegValue(#[from] registry::value::Error),
}

pub struct EventLog {
    handle: HANDLE,
    level: log::Level,
}

unsafe impl Send for EventLog {}
unsafe impl Sync for EventLog {}

#[inline(always)]
fn win_string(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("Failed to create logger")]
    Create(#[from] Error),

    #[error("Failed to set logger")]
    Set(#[from] SetLoggerError),
}

pub fn init(name: &str, level: log::Level) -> Result<(), InitError> {
    let logger = Box::new(EventLog::new(name, level)?);
    log::set_boxed_logger(logger)
        .map(|()| log::set_max_level(LevelFilter::Trace))?;
    Ok(())
}

pub fn deregister(name: &str) -> Result<(), registry::key::Error> {
    let key = Hive::LocalMachine.open(REG_BASEKEY, Security::Read)?;
    key.delete(name, true)
}

pub fn register(name: &str) -> Result<(), Error> {
    let current_exe = std::env::current_exe()?;
    let exe_path = current_exe.to_str().ok_or(Error::ExePathNotFound)?;

    let key = Hive::LocalMachine.open(REG_BASEKEY, Security::Write)?;
    let app_key = key.create(name, Security::Write)?;
    Ok(app_key.set_value(
        "EventMessageFile",
        &Data::String(exe_path.try_into().map_err(|_| Error::ExePathNotFound)?),
    )?)
}

impl EventLog {
    pub fn new(name: &str, level: log::Level) -> Result<EventLog, Error> {
        let wide_name = win_string(name);
        let handle = unsafe { RegisterEventSourceW(null_mut(), wide_name.as_ptr()) };

        if handle.is_null() {
            Err(Error::RegisterSourceFailed(std::io::Error::last_os_error()))
        } else {
            Ok(EventLog { handle, level })
        }
    }
}

impl Drop for EventLog {
    fn drop(&mut self) {
        unsafe { DeregisterEventSource(self.handle) };
    }
}

impl log::Log for EventLog {
    #[inline(always)]
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let (ty, id) = match record.level() {
            Level::Error => (EVENTLOG_ERROR_TYPE, MSG_ERROR),
            Level::Warn => (EVENTLOG_WARNING_TYPE, MSG_WARNING),
            Level::Info => (EVENTLOG_INFORMATION_TYPE, MSG_INFO),
            Level::Debug => (EVENTLOG_INFORMATION_TYPE, MSG_DEBUG),
            Level::Trace => (EVENTLOG_INFORMATION_TYPE, MSG_TRACE),
        };

        let msg = win_string(&format!("{}", record.args()));
        let mut vec = vec![msg.as_ptr()];

        unsafe {
            ReportEventW(
                self.handle,
                ty,
                0,
                id,
                null_mut(),
                vec.len() as u16,
                0,
                vec.as_mut_ptr(),
                null_mut(),
            )
        };
    }

    fn flush(&self) {}
}
