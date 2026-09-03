pub mod db;
use slint::ComponentHandle;

slint::include_modules!();

pub fn create_ui() -> Result<AppWindow, slint::PlatformError> {
    let ui = AppWindow::new()?;
    let ui_handle = ui.as_weak();

    ui.on_request_increase_value(move || {
        if let Some(ui) = ui_handle.upgrade() {
            ui.set_counter(ui.get_counter() + 1);
        }
    });

    Ok(ui)
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).unwrap();
    let ui = create_ui().unwrap();
    ui.run().unwrap();
}
