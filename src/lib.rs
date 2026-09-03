pub mod db;

use db::{Database, UserProfile};
use slint::ComponentHandle;
use std::rc::Rc;

slint::include_modules!();

/// Resolves the platform-appropriate path for the SQLite database file.
#[cfg(not(target_os = "android"))]
fn db_path() -> std::path::PathBuf {
    std::path::PathBuf::from("laporan_bulanan.db")
}

#[cfg(target_os = "android")]
fn db_path() -> std::path::PathBuf {
    // On Android, use the app's internal data directory via an env var set by the runtime.
    let base = std::env::var("ANDROID_DATA").unwrap_or_else(|_| "/data/data".to_string());
    std::path::Path::new(&base)
        .join("com.roguenine.laporan_bulanan")
        .join("laporan_bulanan.db")
}

pub fn create_ui() -> Result<AppWindow, slint::PlatformError> {
    let ui = AppWindow::new()?;
    let db = Rc::new(Database::new(db_path()).expect("Failed to open database"));

    // --- Load existing profile on startup ---
    if let Ok(Some(profile)) = db.get_profile() {
        ui.set_profile_name(profile.name.into());
        ui.set_profile_ni(profile.ni.into());
        ui.set_profile_role(profile.role.into());
        ui.set_profile_work_unit(profile.work_unit.into());
        ui.set_profile_signature_path(
            profile.signature_path.unwrap_or_default().into(),
        );
    }

    // --- Wire up save-profile callback ---
    let db_save = Rc::clone(&db);
    let ui_handle = ui.as_weak();
    ui.on_save_profile(move || {
        let ui = ui_handle.upgrade().expect("UI dropped");
        let profile = UserProfile {
            id: 1,
            name: ui.get_profile_name().to_string(),
            ni: ui.get_profile_ni().to_string(),
            role: ui.get_profile_role().to_string(),
            work_unit: ui.get_profile_work_unit().to_string(),
            signature_path: {
                let p = ui.get_profile_signature_path().to_string();
                if p.is_empty() { None } else { Some(p) }
            },
        };

        match db_save.save_profile(&profile) {
            Ok(_) => ui.set_profile_status_message("Profil berhasil disimpan.".into()),
            Err(e) => ui.set_profile_status_message(
                format!("Gagal menyimpan: {e}").into(),
            ),
        }
    });

    // --- Wire up pick-signature callback (no-op for now; placeholder) ---
    let ui_handle2 = ui.as_weak();
    ui.on_pick_signature(move || {
        // TODO: Integrate a native file picker in a future step.
        let ui = ui_handle2.upgrade().expect("UI dropped");
        ui.set_profile_status_message(
            "Pilih file TTD belum tersedia di versi ini.".into(),
        );
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
