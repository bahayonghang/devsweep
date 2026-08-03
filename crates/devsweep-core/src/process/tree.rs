use std::{
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Default)]
pub(super) struct ProcessTreeBackend {
    #[cfg(windows)]
    job: Option<windows_sys::Win32::Foundation::HANDLE>,
    #[cfg(unix)]
    child_pid: Option<i32>,
}

impl ProcessTreeBackend {
    pub(super) fn configure_command(&mut self, command: &mut Command) {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            // SAFETY: the closure calls only async-signal-safe setpgid between
            // fork and exec and does not capture mutable parent state.
            unsafe {
                command.pre_exec(|| {
                    if libc::setpgid(0, 0) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }
        #[cfg(windows)]
        {
            let _ = command;
            // Job assignment happens in attach() after spawn. CREATE_SUSPENDED
            // is avoided because std::process does not expose the primary
            // thread handle needed to resume cleanly.
        }
        #[cfg(not(any(windows, unix)))]
        {
            let _ = command;
        }
    }

    pub(super) fn attach(&mut self, child: &Child) -> std::io::Result<()> {
        #[cfg(windows)]
        {
            windows_backend::attach_job(self, child)
        }
        #[cfg(unix)]
        {
            self.child_pid = Some(child.id() as i32);
            Ok(())
        }
        #[cfg(not(any(windows, unix)))]
        {
            let _ = child;
            Ok(())
        }
    }

    pub(super) fn terminate(&mut self, child: &mut Child) {
        #[cfg(windows)]
        {
            windows_backend::terminate_job(self);
            let _ = child.kill();
        }
        #[cfg(unix)]
        {
            if let Some(pid) = self.child_pid.take() {
                // SAFETY: pid is the positive child id captured after spawn;
                // negating it addresses only the process group created above.
                unsafe {
                    let _ = libc::kill(-pid, libc::SIGKILL);
                }
            }
            let _ = child.kill();
        }
        #[cfg(not(any(windows, unix)))]
        {
            let _ = child.kill();
        }
    }
}

impl Drop for ProcessTreeBackend {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            windows_backend::close_job(self);
        }
    }
}

pub(super) fn wait_for_exit(child: &mut Child, grace: Duration) {
    let deadline = Instant::now() + grace;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
        }
    }
}

#[cfg(windows)]
mod windows_backend {
    use super::ProcessTreeBackend;
    use std::{io, mem, process::Child};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
        System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject, TerminateJobObject,
        },
    };

    pub(super) fn attach_job(backend: &mut ProcessTreeBackend, child: &Child) -> io::Result<()> {
        use std::os::windows::io::AsRawHandle;

        // SAFETY: the created job handle is owned by backend after successful
        // assignment and is closed on each failure or by terminate/drop.
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() || job == INVALID_HANDLE_VALUE {
                return Err(io::Error::last_os_error());
            }

            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if ok == 0 {
                let err = io::Error::last_os_error();
                CloseHandle(job);
                return Err(err);
            }

            let process_handle = child.as_raw_handle() as HANDLE;
            let ok = AssignProcessToJobObject(job, process_handle);
            if ok == 0 {
                let err = io::Error::last_os_error();
                CloseHandle(job);
                return Err(err);
            }

            backend.job = Some(job);
            Ok(())
        }
    }

    pub(super) fn terminate_job(backend: &mut ProcessTreeBackend) {
        if let Some(job) = backend.job.take() {
            // SAFETY: taking the option transfers the one owned live handle;
            // it is terminated and closed exactly once here.
            unsafe {
                let _ = TerminateJobObject(job, 1);
                let _ = CloseHandle(job);
            }
        }
    }

    pub(super) fn close_job(backend: &mut ProcessTreeBackend) {
        if let Some(job) = backend.job.take() {
            // SAFETY: taking the option transfers the one owned live handle,
            // which is closed exactly once by this drop path.
            unsafe {
                let _ = CloseHandle(job);
            }
        }
    }
}

/// Returns true when a process id appears still alive.
/// Used by dynamic tree-cleanup tests; failure to prove death is a test No-Go.
#[cfg(test)]
pub(super) fn process_is_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        windows_process_is_alive(pid)
    }
    #[cfg(unix)]
    {
        // SAFETY: signal 0 performs a liveness probe and does not mutate the
        // process; pid comes from fixture output parsed as a positive u32.
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = pid;
        false
    }
}

#[cfg(all(test, windows))]
fn windows_process_is_alive(pid: u32) -> bool {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, STILL_ACTIVE, WAIT_TIMEOUT},
        System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
            PROCESS_SYNCHRONIZE, WaitForSingleObject,
        },
    };

    // SAFETY: OpenProcess returns an owned query handle for pid; it is checked
    // before use and closed exactly once after the read-only liveness queries.
    unsafe {
        let handle = OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            0,
            pid,
        );
        if handle.is_null() {
            return false;
        }
        let mut exit_code = 0_u32;
        let ok = GetExitCodeProcess(handle, &mut exit_code);
        let wait = WaitForSingleObject(handle, 0);
        CloseHandle(handle);
        ok != 0 && exit_code == STILL_ACTIVE as u32 && wait == WAIT_TIMEOUT
    }
}
