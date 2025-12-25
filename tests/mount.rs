use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tempdir::TempDir;

#[test]
fn mount() {
    let tmp_dir = TempDir::new("testdir").unwrap();
    let mountpoint = tmp_dir.path();

    let mut child = Command::new(env!("CARGO_BIN_EXE_rusty-file-system"))
        .arg(mountpoint)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to mount filesystem");

    thread::sleep(Duration::from_secs(3));

    child.kill().ok();
    let status = child.wait().unwrap();
    assert!(!status.success(), "filesystem exited unexpectedly");
}
