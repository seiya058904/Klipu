#[cfg(target_os = "windows")]
include!("probe/battery.rs");

#[cfg(target_os = "windows")]
include!("probe/restoring.rs");
