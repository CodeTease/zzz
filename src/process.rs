use sysinfo::{Pid, System};

pub fn is_process_running(pid: u32) -> bool {
    let mut sys = System::new();
    let sys_pid = Pid::from(pid as usize);
    sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[sys_pid]), true);
    sys.process(sys_pid).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_process_running() {
        let my_pid = std::process::id();
        assert!(is_process_running(my_pid));
        assert!(!is_process_running(999999));
    }
}
