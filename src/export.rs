use rust_xlsxwriter::{Format, Workbook};
use crate::db::{DailyLog, Database};

pub fn export_excel(db: &Database, year: i32, month: u32, filepath: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut workbook = Workbook::new();
    
    // Add formats
    let title_format = Format::new().set_bold().set_font_size(14);
    let header_format = Format::new().set_bold().set_border(rust_xlsxwriter::FormatBorder::Thin);
    let normal_format = Format::new().set_border(rust_xlsxwriter::FormatBorder::Thin);
    
    // Profile
    let profile = db.get_profile()?.unwrap_or_default();
    
    // Format year_month for querying
    let year_month = format!("{:04}-{:02}", year, month);
    let logs = db.get_logs_for_month(&year_month)?;
    
    // Create map for easy lookup
    use std::collections::HashMap;
    let log_map: HashMap<String, &DailyLog> = logs.iter().map(|l| (l.date.clone(), l)).collect();
    
    // Calculate start and end day of the month
    use chrono::{NaiveDate, Datelike, Weekday};
    let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let next_month = if month == 12 { 1 } else { month + 1 };
    let next_month_year = if month == 12 { year + 1 } else { year };
    let last_day = NaiveDate::from_ymd_opt(next_month_year, next_month, 1).unwrap().pred_opt().unwrap();
    
    // Sheet 1: Lembar Kerja Harian
    let sheet1 = workbook.add_worksheet().set_name("Lembar Kerja Harian")?;
    sheet1.write_string_with_format(0, 0, "CATATAN LEMBAR KERJA HARIAN", &title_format)?;
    
    sheet1.write_string(2, 0, "Nama")?; sheet1.write_string(2, 1, &format!(": {}", profile.name))?;
    sheet1.write_string(3, 0, "NI")?; sheet1.write_string(3, 1, &format!(": {}", profile.ni))?;
    sheet1.write_string(4, 0, "Jabatan")?; sheet1.write_string(4, 1, &format!(": {}", profile.role))?;
    sheet1.write_string(5, 0, "Periode")?; sheet1.write_string(5, 1, &format!(": {}", year_month))?;
    sheet1.write_string(6, 0, "Unit Kerja")?; sheet1.write_string(6, 1, &format!(": {}", profile.work_unit))?;
    
    let mut row = 8;
    sheet1.write_string_with_format(row, 0, "Tanggal", &header_format)?;
    sheet1.write_string_with_format(row, 1, "Hari", &header_format)?;
    sheet1.write_string_with_format(row, 2, "Kegiatan", &header_format)?;
    sheet1.write_string_with_format(row, 3, "Output", &header_format)?;
    sheet1.write_string_with_format(row, 4, "Keterangan", &header_format)?;
    
    sheet1.set_column_width(0, 15)?;
    sheet1.set_column_width(1, 10)?;
    sheet1.set_column_width(2, 30)?;
    sheet1.set_column_width(3, 30)?;
    sheet1.set_column_width(4, 20)?;
    
    row += 1;
    let mut current = first_day;
    while current <= last_day {
        let date_str = current.format("%Y-%m-%d").to_string();
        let is_weekend = matches!(current.weekday(), Weekday::Sat | Weekday::Sun);
        let day_name = crate::day_name_id(current.weekday());
        
        let mut keg = String::new();
        let mut out = String::new();
        let mut ket = String::new();
        
        if is_weekend {
            keg = "-".to_string();
            out = "-".to_string();
            ket = "Libur".to_string();
        }
        
        if let Some(log) = log_map.get(&date_str) {
            if let Some(desc) = &log.activity_desc { keg = desc.clone(); }
            if let Some(output) = &log.activity_output { out = output.clone(); }
            if let Some(note) = &log.activity_note { ket = note.clone(); }
        }
        
        sheet1.write_string_with_format(row, 0, &date_str, &normal_format)?;
        sheet1.write_string_with_format(row, 1, day_name, &normal_format)?;
        sheet1.write_string_with_format(row, 2, &keg, &normal_format)?;
        sheet1.write_string_with_format(row, 3, &out, &normal_format)?;
        sheet1.write_string_with_format(row, 4, &ket, &normal_format)?;
        
        row += 1;
        current = current.succ_opt().unwrap();
    }
    
    row += 2;
    sheet1.write_string(row, 3, &profile.role)?;
    row += 1;
    sheet1.write_string(row, 3, "TTD")?;
    row += 4;
    sheet1.write_string(row, 3, &profile.name)?;
    row += 1;
    sheet1.write_string(row, 3, &profile.ni)?;
    
    // Sheet 2: Lembar Absensi Harian
    let sheet2 = workbook.add_worksheet().set_name("Lembar Absensi Harian")?;
    sheet2.write_string_with_format(0, 0, "CATATAN LEMBAR ABSENSI HARIAN", &title_format)?;
    
    sheet2.write_string(2, 0, "Nama")?; sheet2.write_string(2, 1, &format!(": {}", profile.name))?;
    sheet2.write_string(3, 0, "NI")?; sheet2.write_string(3, 1, &format!(": {}", profile.ni))?;
    sheet2.write_string(4, 0, "Jabatan")?; sheet2.write_string(4, 1, &format!(": {}", profile.role))?;
    sheet2.write_string(5, 0, "Periode")?; sheet2.write_string(5, 1, &format!(": {}", year_month))?;
    sheet2.write_string(6, 0, "Unit Kerja")?; sheet2.write_string(6, 1, &format!(": {}", profile.work_unit))?;
    
    let mut row = 8;
    sheet2.write_string_with_format(row, 0, "Tanggal", &header_format)?;
    sheet2.write_string_with_format(row, 1, "Hari", &header_format)?;
    sheet2.write_string_with_format(row, 2, "Masuk", &header_format)?;
    sheet2.write_string_with_format(row, 3, "Pulang", &header_format)?;
    sheet2.write_string_with_format(row, 4, "Total Jam", &header_format)?;
    sheet2.write_string_with_format(row, 5, "Paraf", &header_format)?;
    sheet2.write_string_with_format(row, 6, "Ket.", &header_format)?;
    
    sheet2.set_column_width(0, 15)?;
    sheet2.set_column_width(1, 10)?;
    sheet2.set_column_width(2, 10)?;
    sheet2.set_column_width(3, 10)?;
    sheet2.set_column_width(4, 15)?;
    sheet2.set_column_width(5, 10)?;
    sheet2.set_column_width(6, 20)?;
    
    row += 1;
    let mut current = first_day;
    while current <= last_day {
        let date_str = current.format("%Y-%m-%d").to_string();
        let is_weekend = matches!(current.weekday(), Weekday::Sat | Weekday::Sun);
        let day_name = crate::day_name_id(current.weekday());
        
        let mut masuk = String::new();
        let mut pulang = String::new();
        let mut total = String::new();
        let paraf = String::new();
        let mut ket = String::new();
        
        if is_weekend {
            ket = "Libur".to_string();
        }
        
        if let Some(log) = log_map.get(&date_str) {
            if let Some(cin) = &log.check_in { masuk = cin.clone(); }
            if let Some(cout) = &log.check_out { pulang = cout.clone(); }
            total = crate::total_hours_str(&masuk, &pulang);
            if let Some(note) = &log.attendance_note { ket = note.clone(); }
        }
        
        sheet2.write_string_with_format(row, 0, &date_str, &normal_format)?;
        sheet2.write_string_with_format(row, 1, day_name, &normal_format)?;
        sheet2.write_string_with_format(row, 2, &masuk, &normal_format)?;
        sheet2.write_string_with_format(row, 3, &pulang, &normal_format)?;
        sheet2.write_string_with_format(row, 4, &total, &normal_format)?;
        sheet2.write_string_with_format(row, 5, &paraf, &normal_format)?;
        sheet2.write_string_with_format(row, 6, &ket, &normal_format)?;
        
        row += 1;
        current = current.succ_opt().unwrap();
    }
    
    row += 2;
    sheet2.write_string(row, 4, &profile.role)?;
    row += 1;
    sheet2.write_string(row, 4, "TTD")?;
    row += 4;
    sheet2.write_string(row, 4, &profile.name)?;
    row += 1;
    sheet2.write_string(row, 4, &profile.ni)?;
    
    // Sheet 3: Dokumentasi
    let sheet3 = workbook.add_worksheet().set_name("Dokumentasi")?;
    sheet3.write_string_with_format(0, 0, "DOKUMENTASI HARIAN", &title_format)?;
    
    let mut row = 2;
    // We only iterate through logs that have photos
    for log in logs.iter() {
        if let Some(photo_path) = &log.photo_path {
            if std::path::Path::new(photo_path).exists() {
                sheet3.write_string(row, 0, &log.date)?;
                row += 1;
                
                let image = rust_xlsxwriter::Image::new(photo_path)?;
                sheet3.insert_image(row, 0, &image)?;
                // The height of the row depends on image, approx let's just jump by a fixed number of rows
                row += 15; 
            }
        }
    }
    
    workbook.save(filepath)?;
    
    Ok(())
}
