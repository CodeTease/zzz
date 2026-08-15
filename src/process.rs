pub fn is_process_running(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
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
