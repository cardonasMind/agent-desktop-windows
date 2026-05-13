#[cfg(target_os = "windows")]
mod tree;
#[cfg(target_os = "windows")]
mod input;
#[cfg(target_os = "windows")]
mod system;
#[cfg(target_os = "windows")]
mod actions;

#[cfg(target_os = "windows")]
mod adapter;

#[cfg(target_os = "windows")]
pub use adapter::WindowsAdapter;

// Stub for non-Windows compilation (CI, cross-compile checks)
#[cfg(not(target_os = "windows"))]
mod stub {
    use agent_desktop_core::adapter::PlatformAdapter;

    pub struct WindowsAdapter;

    impl WindowsAdapter {
        pub fn new() -> Self { Self }
    }

    impl Default for WindowsAdapter {
        fn default() -> Self { Self::new() }
    }

    impl PlatformAdapter for WindowsAdapter {}
}

#[cfg(not(target_os = "windows"))]
pub use stub::WindowsAdapter;
