#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // webkit2gtk's DMABUF renderer crashes with "Wayland protocol error 71"
    // on several Wayland/GPU stacks (observed on the Fedora target machine).
    // Disable it unless the user has explicitly configured otherwise.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    lian_app_lib::run();
}
