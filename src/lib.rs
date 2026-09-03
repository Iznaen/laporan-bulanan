pub mod db;
pub mod export;

use chrono::{Datelike, Local, NaiveDate, Weekday};
use db::{DailyLog, Database, UserProfile};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

slint::include_modules!();

// ── Platform path helpers ──────────────────────────────────────────────────

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

/// Fixed path for the single signature image managed by the app.
/// Picking a new signature always overwrites this one file.
fn signature_file_path() -> std::path::PathBuf {
    db_path().with_file_name("signature.png")
}

// ── Date/time helpers ──────────────────────────────────────────────────────

pub fn day_name_id(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "Senin",
        Weekday::Tue => "Selasa",
        Weekday::Wed => "Rabu",
        Weekday::Thu => "Kamis",
        Weekday::Fri => "Jumat",
        Weekday::Sat => "Sabtu",
        Weekday::Sun => "Minggu",
    }
}

fn month_name_id(m: u32) -> &'static str {
    match m {
        1 => "Januari", 2 => "Februari", 3 => "Maret",
        4 => "April", 5 => "Mei", 6 => "Juni",
        7 => "Juli", 8 => "Agustus", 9 => "September",
        10 => "Oktober", 11 => "November", 12 => "Desember",
        _ => "",
    }
}

/// "Senin, 1 September 2026"
fn date_display_with_day(date: NaiveDate) -> String {
    format!(
        "{}, {} {} {}",
        day_name_id(date.weekday()),
        date.day(),
        month_name_id(date.month()),
        date.year()
    )
}

/// "September 2026"
fn period_label(year: i32, month: u32) -> String {
    format!("{} {}", month_name_id(month), year)
}

/// Computes "X jam Y menit" from "HH:MM" check-in/out strings.
/// Returns an empty string if either input is missing or unparseable.
pub fn total_hours_str(check_in: &str, check_out: &str) -> String {
    if check_in.is_empty() || check_out.is_empty() {
        return String::new();
    }
    let parse_mins = |s: &str| -> Option<i64> {
        let mut parts = s.splitn(2, ':');
        let h: i64 = parts.next()?.parse().ok()?;
        let m: i64 = parts.next()?.parse().ok()?;
        Some(h * 60 + m)
    };
    let in_m = match parse_mins(check_in) { Some(v) => v, None => return String::new() };
    let out_m = match parse_mins(check_out) { Some(v) => v, None => return String::new() };
    let mut diff = out_m - in_m;
    if diff < 0 { diff += 24 * 60; } // handle overnight shift
    format!("{} jam {} menit", diff / 60, diff % 60)
}

/// Converts an empty string to None; non-empty strings are wrapped in Some.
fn non_empty(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

/// Returns true if `s` is empty (not yet filled) or a valid HH:MM time string.
/// Returns false only when `s` is non-empty but not a parseable H:MM / HH:MM value.
fn is_valid_time(s: &str) -> bool {
    if s.is_empty() { return true; }
    let mut parts = s.splitn(2, ':');
    match (
        parts.next().and_then(|h| h.parse::<u32>().ok()),
        parts.next().and_then(|m| m.parse::<u32>().ok()),
    ) {
        (Some(h), Some(m)) => h < 24 && m < 60,
        _ => false,
    }
}

// ── UI helper functions ────────────────────────────────────────────────────

/// Updates the daily-date, daily-date-display, and daily-is-weekend properties on the UI.
fn set_daily_date(ui: &AppWindow, date: NaiveDate) {
    let is_weekend = matches!(date.weekday(), Weekday::Sat | Weekday::Sun);
    ui.set_daily_date(date.format("%Y-%m-%d").to_string().into());
    ui.set_daily_date_display(date_display_with_day(date).into());
    ui.set_daily_is_weekend(is_weekend);
}

/// Clears all daily log fields, then populates them from the DB for the given date.
fn load_daily_log(db: &Database, date_str: &str, ui: &AppWindow) {
    ui.set_daily_check_in("".into());
    ui.set_daily_check_out("".into());
    ui.set_daily_total_hours("".into());
    ui.set_daily_total_hours_error("".into());
    ui.set_daily_attendance_note("".into());
    ui.set_daily_activity_desc("".into());
    ui.set_daily_activity_output("".into());
    ui.set_daily_activity_note("".into());
    ui.set_daily_photo_path("".into());
    ui.set_daily_status_message("".into());

    if let Ok(Some(log)) = db.get_daily_log(date_str) {
        let check_in = log.check_in.unwrap_or_default();
        let check_out = log.check_out.unwrap_or_default();
        let total = total_hours_str(&check_in, &check_out);
        ui.set_daily_check_in(check_in.into());
        ui.set_daily_check_out(check_out.into());
        ui.set_daily_total_hours(total.into());
        ui.set_daily_attendance_note(log.attendance_note.unwrap_or_default().into());
        ui.set_daily_activity_desc(log.activity_desc.unwrap_or_default().into());
        ui.set_daily_activity_output(log.activity_output.unwrap_or_default().into());
        ui.set_daily_activity_note(log.activity_note.unwrap_or_default().into());
        ui.set_daily_photo_path(log.photo_path.unwrap_or_default().into());
    }
}

/// Builds and pushes the history entries model for the given month/year.
fn load_history(db: &Database, year: i32, month: u32, ui: &AppWindow) {
    let first_day = match NaiveDate::from_ymd_opt(year, month, 1) {
        Some(d) => d, None => return,
    };
    let last_day = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).and_then(|d| d.pred_opt())
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).and_then(|d| d.pred_opt())
    };
    let last_day = match last_day { Some(d) => d, None => return };

    let year_month = format!("{:04}-{:02}", year, month);
    let db_logs = db.get_logs_for_month(&year_month).unwrap_or_default();

    // Build a lookup map: date string → DailyLog
    let log_map: std::collections::HashMap<String, &DailyLog> =
        db_logs.iter().map(|l| (l.date.clone(), l)).collect();

    let mut entries = Vec::new();
    let mut current = first_day;
    while current <= last_day {
        let date_str = current.format("%Y-%m-%d").to_string();
        let is_weekend = matches!(current.weekday(), Weekday::Sat | Weekday::Sun);
        let date_display = format!("{} {}", current.day(), month_name_id(month));
        let day_name = day_name_id(current.weekday()).to_owned();

        let entry = if let Some(log) = log_map.get(&date_str) {
            let check_in = log.check_in.clone().unwrap_or_default();
            let check_out = log.check_out.clone().unwrap_or_default();
            let total = total_hours_str(&check_in, &check_out);
            let has_data = log.check_in.is_some() || log.activity_desc.is_some();
            DailyLogEntry {
                date: date_str.into(),
                date_display: date_display.into(),
                day_name: day_name.into(),
                check_in: check_in.into(),
                check_out: check_out.into(),
                total_hours: total.into(),
                activity_desc: log.activity_desc.clone().unwrap_or_default().into(),
                is_weekend,
                has_data,
            }
        } else {
            DailyLogEntry {
                date: date_str.into(),
                date_display: date_display.into(),
                day_name: day_name.into(),
                check_in: "".into(),
                check_out: "".into(),
                total_hours: "".into(),
                activity_desc: "".into(),
                is_weekend,
                has_data: false,
            }
        };

        entries.push(entry);
        current = match current.succ_opt() { Some(d) => d, None => break };
    }

    let model = Rc::new(VecModel::from(entries));
    ui.set_history_entries(ModelRc::from(model));
    ui.set_history_period_label(period_label(year, month).into());
    ui.set_history_export_status("".into());
}

// ── Public entry point ─────────────────────────────────────────────────────

pub fn create_ui() -> Result<AppWindow, slint::PlatformError> {
    let ui = AppWindow::new()?;
    let db = Rc::new(Database::new(db_path()).expect("Failed to open database"));
    let today = Local::now().date_naive();

    // ── Profile: load saved data ─────────────────────────────────────────
    let sig_path = signature_file_path().to_string_lossy().to_string();
    if let Ok(Some(profile)) = db.get_profile() {
        ui.set_profile_name(profile.name.into());
        ui.set_profile_ni(profile.ni.into());
        ui.set_profile_role(profile.role.into());
        ui.set_profile_work_unit(profile.work_unit.into());
    }
    ui.set_profile_signature_path(sig_path.clone().into());

    // ── Daily: initialize to today ───────────────────────────────────────
    set_daily_date(&ui, today);
    load_daily_log(&db, &today.format("%Y-%m-%d").to_string(), &ui);

    // ── History: initialize to current month ─────────────────────────────
    // Rc<RefCell<(year, month)>> tracks which month the history screen is showing.
    let history_month = Rc::new(RefCell::new((today.year(), today.month())));
    load_history(&db, today.year(), today.month(), &ui);

    // ── Profile callbacks ────────────────────────────────────────────────
    {
        let db_ref = Rc::clone(&db);
        let ui_h = ui.as_weak();
        let sig = sig_path.clone();
        ui.on_save_profile(move || {
            let ui = ui_h.upgrade().expect("UI dropped");
            let profile = UserProfile {
                id: 1,
                name: ui.get_profile_name().to_string(),
                ni: ui.get_profile_ni().to_string(),
                role: ui.get_profile_role().to_string(),
                work_unit: ui.get_profile_work_unit().to_string(),
                signature_path: Some(sig.clone()),
            };
            match db_ref.save_profile(&profile) {
                Ok(_) => ui.set_profile_status_message("Profil berhasil disimpan.".into()),
                Err(e) => ui.set_profile_status_message(format!("Gagal menyimpan: {e}").into()),
            }
        });
    }
    {
        let ui_h = ui.as_weak();
        ui.on_pick_signature(move || {
            // TODO: integrate a native file picker.
            let ui = ui_h.upgrade().expect("UI dropped");
            ui.set_profile_status_message("Pilih file TTD belum tersedia di versi ini.".into());
        });
    }

    // ── Daily callbacks ──────────────────────────────────────────────────
    {
        let db_ref = Rc::clone(&db);
        let ui_h = ui.as_weak();
        ui.on_save_daily_log(move || {
            let ui = ui_h.upgrade().expect("UI dropped");
            let check_in = ui.get_daily_check_in().to_string();
            let check_out = ui.get_daily_check_out().to_string();
            let total = total_hours_str(&check_in, &check_out);
            // Show computed total in UI immediately.
            ui.set_daily_total_hours(total.into());

            let log = DailyLog {
                date: ui.get_daily_date().to_string(),
                check_in: non_empty(check_in),
                check_out: non_empty(check_out),
                attendance_note: non_empty(ui.get_daily_attendance_note().to_string()),
                activity_desc: non_empty(ui.get_daily_activity_desc().to_string()),
                activity_output: non_empty(ui.get_daily_activity_output().to_string()),
                activity_note: non_empty(ui.get_daily_activity_note().to_string()),
                photo_path: non_empty(ui.get_daily_photo_path().to_string()),
            };
            match db_ref.save_daily_log(&log) {
                Ok(_) => ui.set_daily_status_message("Catatan harian berhasil disimpan.".into()),
                Err(e) => ui.set_daily_status_message(format!("Gagal menyimpan: {e}").into()),
            }
        });
    }
    {
        let db_ref = Rc::clone(&db);
        let ui_h = ui.as_weak();
        ui.on_navigate_prev_day(move || {
            let ui = ui_h.upgrade().expect("UI dropped");
            if let Ok(date) = NaiveDate::parse_from_str(&ui.get_daily_date(), "%Y-%m-%d") {
                if let Some(prev) = date.pred_opt() {
                    set_daily_date(&ui, prev);
                    load_daily_log(&db_ref, &prev.format("%Y-%m-%d").to_string(), &ui);
                }
            }
        });
    }
    {
        let db_ref = Rc::clone(&db);
        let ui_h = ui.as_weak();
        ui.on_navigate_next_day(move || {
            let ui = ui_h.upgrade().expect("UI dropped");
            if let Ok(date) = NaiveDate::parse_from_str(&ui.get_daily_date(), "%Y-%m-%d") {
                if let Some(next) = date.succ_opt() {
                    set_daily_date(&ui, next);
                    load_daily_log(&db_ref, &next.format("%Y-%m-%d").to_string(), &ui);
                }
            }
        });
    }
    {
        let ui_h = ui.as_weak();
        ui.on_pick_photo(move || {
            // TODO: integrate a native file picker.
            let ui = ui_h.upgrade().expect("UI dropped");
            ui.set_daily_status_message("Pilih foto belum tersedia di versi ini.".into());
        });
    }
    {
        // Recalculate Total Jam Kerja live on every Jam Masuk / Jam Pulang keystroke.
        // Shows a format error when either field is non-empty but not a valid HH:MM value.
        let ui_h = ui.as_weak();
        ui.on_recalculate_hours(move || {
            let ui = ui_h.upgrade().expect("UI dropped");
            let ci = ui.get_daily_check_in().to_string();
            let co = ui.get_daily_check_out().to_string();

            if !is_valid_time(&ci) || !is_valid_time(&co) {
                // One or both fields have invalid format — show warning, clear total.
                ui.set_daily_total_hours("".into());
                ui.set_daily_total_hours_error(
                    "⚠ Format Jam Masuk/Keluar tidak sesuai (gunakan HH:MM, cth: 07:00)".into(),
                );
            } else {
                // Both fields are empty or valid — calculate (empty inputs → empty total).
                ui.set_daily_total_hours(total_hours_str(&ci, &co).into());
                ui.set_daily_total_hours_error("".into());
            }
        });
    }

    // ── History callbacks ────────────────────────────────────────────────
    {
        let db_ref = Rc::clone(&db);
        let hm = Rc::clone(&history_month);
        let ui_h = ui.as_weak();
        ui.on_navigate_prev_month(move || {
            let ui = ui_h.upgrade().expect("UI dropped");
            let mut m = hm.borrow_mut();
            if m.1 == 1 { m.0 -= 1; m.1 = 12; } else { m.1 -= 1; }
            load_history(&db_ref, m.0, m.1, &ui);
        });
    }
    {
        let db_ref = Rc::clone(&db);
        let hm = Rc::clone(&history_month);
        let ui_h = ui.as_weak();
        ui.on_navigate_next_month(move || {
            let ui = ui_h.upgrade().expect("UI dropped");
            let mut m = hm.borrow_mut();
            if m.1 == 12 { m.0 += 1; m.1 = 1; } else { m.1 += 1; }
            load_history(&db_ref, m.0, m.1, &ui);
        });
    }
    {
        let db_ref = Rc::clone(&db);
        let ui_h = ui.as_weak();
        ui.on_history_entry_selected(move |date| {
            let ui = ui_h.upgrade().expect("UI dropped");
            let date_str = date.to_string();
            if let Ok(d) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                set_daily_date(&ui, d);
                load_daily_log(&db_ref, &date_str, &ui);
                ui.set_active_tab(Tab::Daily);
            }
        });
    }

    {
        let db_ref = Rc::clone(&db);
        let hm = Rc::clone(&history_month);
        let ui_h = ui.as_weak();
        ui.on_history_export_excel(move || {
            let ui = ui_h.upgrade().expect("UI dropped");
            ui.set_history_export_status("Mengekspor...".into());
            
            let m = hm.borrow();
            let year = m.0;
            let month = m.1;
            
            let filename = format!("laporan_bulanan_{}_{:02}.xlsx", year, month);
            // Default to current directory for desktop. Android would need something else.
            let mut filepath = std::env::current_dir().unwrap_or_default();
            filepath.push(&filename);
            
            match crate::export::export_excel(&db_ref, year, month, filepath.to_str().unwrap_or(&filename)) {
                Ok(_) => {
                    ui.set_history_export_status(format!("Berhasil diekspor ke {}", filename).into());
                }
                Err(e) => {
                    ui.set_history_export_status(format!("Gagal: {}", e).into());
                }
            }
        });
    }

    Ok(ui)
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).unwrap();
    let ui = create_ui().unwrap();
    ui.run().unwrap();
}
