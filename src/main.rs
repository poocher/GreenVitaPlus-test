use vita_newlib_shims as _;

mod api;
mod api_xbox;
mod app;
mod i18n;
mod input;
mod jobs;
mod safe_memory;
mod settings;
mod shell;
mod streaming;

use api_xbox::api::{
    ApiClient, ApiClientConfig, Console, ConsolesResponse, StreamKind, WaitTimeResponse,
};
use api_xbox::auth::{DeviceCodeAuth, DeviceCodePoll, MsalAuth, StreamingCredentials, XboxProfile};
use api_xbox::stream::{Stream, StreamState};
use app::{App, AppCommand, AppState, InputCommand, NavigationCommand};
use settings::Locale;

#[used]
#[unsafe(export_name = "sceUserMainThreadStackSize")]
pub static SCE_USER_MAIN_THREAD_STACK_SIZE: u32 = 2 * 1024 * 1024;

#[used]
#[unsafe(export_name = "sceLibcHeapSize")]
pub static SCE_LIBC_HEAP_SIZE: u32 = 32 * 1024 * 1024;

#[used]
#[unsafe(export_name = "_newlib_heap_size_user")]
pub static NEWLIB_HEAP_SIZE_USER: u32 = 192 * 1024 * 1024;

mod fs_utils {
    use anyhow::{Context, Result};

    /// Removes `path` before writing - `std::fs::write` alone doesn't reliably truncate an
    /// existing file on the Vita's newlib filesystem.
    pub fn write_file_truncating(path: &str, data: impl AsRef<[u8]>) -> Result<()> {
        let _ = std::fs::remove_file(path);
        std::fs::write(path, data).with_context(|| format!("failed to write {path}"))
    }
}

/// Set CPU/GPU/bus clocks to their safe maximum within Sony's official dynamic range.
/// 444 MHz ARM / 222 MHz GPU / 222 MHz bus are the same speeds used by demanding retail
/// games (Killzone: Mercenary, Borderlands 2). This does NOT exceed the chip's rated specs.
#[cfg(target_os = "vita")]
fn apply_clock_boost() {
    unsafe {
        let cpu = vitasdk_sys::scePowerSetArmClockFrequency(444);
        let gpu = vitasdk_sys::scePowerSetGpuClockFrequency(222);
        let bus = vitasdk_sys::scePowerSetBusClockFrequency(222);
        eprintln!("Clock boost applied: CPU=444MHz (ret={cpu:#x}), GPU=222MHz (ret={gpu:#x}), BUS=222MHz (ret={bus:#x})");
    }
}

#[cfg(not(target_os = "vita"))]
fn apply_clock_boost() {
    // No-op on non-Vita platforms (development builds).
}

fn main() -> anyhow::Result<()> {
    let _app_util = safe_memory::AppUtil::initialize()?;

    // Apply clock boost early — before App::new() — so the entire init benefits.
    let settings = settings::Settings::load();
    if settings.boost_clocks {
        apply_clock_boost();
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let app = App::new()?;
        shell::run(app).await
    })
}
