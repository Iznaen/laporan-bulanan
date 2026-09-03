use genpdf::{Alignment, Document, Element, elements, fonts, style};
use crate::db::{DailyLog, Database};
use chrono::{Datelike, NaiveDate, Weekday};

pub fn export_pdf(db: &Database, year: i32, month: u32, filepath: &str) -> Result<(), Box<dyn std::error::Error>> {
    let regular = fonts::FontData::new(include_bytes!("../assets/fonts/LiberationSans-Regular.ttf").to_vec(), None).unwrap();
    let bold = fonts::FontData::new(include_bytes!("../assets/fonts/LiberationSans-Bold.ttf").to_vec(), None).unwrap();
    let italic = fonts::FontData::new(include_bytes!("../assets/fonts/LiberationSans-Italic.ttf").to_vec(), None).unwrap();
    let bold_italic = fonts::FontData::new(include_bytes!("../assets/fonts/LiberationSans-BoldItalic.ttf").to_vec(), None).unwrap();

    let font_family = fonts::FontFamily {
        regular,
        bold,
        italic,
        bold_italic,
    };

    let mut doc = Document::new(font_family);
    doc.set_title("Laporan Bulanan");
    
    // Add page margins
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(15); // 15mm margins
    doc.set_page_decorator(decorator);
    
    // Default styling
    let mut default_style = style::Style::new();
    default_style.set_font_size(10);
    
    let mut bold_style = default_style.clone();
    bold_style.set_bold();
    
    let mut title_style = default_style.clone();
    title_style.set_bold();
    title_style.set_font_size(14);

    // Profile
    let profile = db.get_profile()?.unwrap_or_default();
    let year_month = format!("{:04}-{:02}", year, month);
    let logs = db.get_logs_for_month(&year_month)?;
    
    use std::collections::HashMap;
    let log_map: HashMap<String, &DailyLog> = logs.iter().map(|l| (l.date.clone(), l)).collect();
    
    let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let next_month = if month == 12 { 1 } else { month + 1 };
    let next_month_year = if month == 12 { year + 1 } else { year };
    let last_day = NaiveDate::from_ymd_opt(next_month_year, next_month, 1).unwrap().pred_opt().unwrap();
    
    // Helper to create styled paragraphs
    let text = |s: &str| -> elements::StyledElement<elements::Paragraph> {
        elements::Paragraph::new(s).styled(default_style.clone())
    };
    let bold_text = |s: &str| -> elements::StyledElement<elements::Paragraph> {
        elements::Paragraph::new(s).styled(bold_style.clone())
    };
    let title_text = |s: &str| -> elements::StyledElement<elements::Paragraph> {
        elements::Paragraph::new(s).aligned(Alignment::Center).styled(title_style.clone())
    };
    let right_text = |s: &str| -> elements::StyledElement<elements::Paragraph> {
        elements::Paragraph::new(s).aligned(Alignment::Right).styled(default_style.clone())
    };
    
    // Helper to box elements for tables
    let boxed = |e: elements::StyledElement<elements::Paragraph>| -> Box<dyn Element> {
        Box::new(e)
    };
    
    // --- Sheet 1: Lembar Kerja Harian ---
    doc.push(title_text("CATATAN LEMBAR KERJA HARIAN"));
    doc.push(elements::Break::new(1));
    
    let profile_lines = vec![
        format!("Nama       : {}", profile.name),
        format!("NI         : {}", profile.ni),
        format!("Jabatan    : {}", profile.role),
        format!("Periode    : {}", year_month),
        format!("Unit Kerja : {}", profile.work_unit),
    ];
    for line in profile_lines.clone() {
        doc.push(text(&line));
    }
    doc.push(elements::Break::new(1));
    
    let mut table1 = elements::TableLayout::new(vec![2, 2, 4, 4, 3]);
    table1.set_cell_decorator(elements::FrameCellDecorator::new(true, true, false));
    let _ = table1.push_row(vec![
        boxed(bold_text("Tanggal")),
        boxed(bold_text("Hari")),
        boxed(bold_text("Kegiatan")),
        boxed(bold_text("Output")),
        boxed(bold_text("Keterangan")),
    ]);
    
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
        
        let _ = table1.push_row(vec![
            boxed(text(&date_str)),
            boxed(text(day_name)),
            boxed(text(&keg)),
            boxed(text(&out)),
            boxed(text(&ket)),
        ]);
        current = current.succ_opt().unwrap();
    }
    doc.push(table1);
    doc.push(elements::Break::new(2));
    
    // TTD Block
    let mut ttd_layout = elements::LinearLayout::vertical();
    ttd_layout.push(right_text(&profile.role));
    ttd_layout.push(elements::Break::new(3));
    ttd_layout.push(right_text(&profile.name));
    ttd_layout.push(right_text(&profile.ni));
    doc.push(ttd_layout);
    
    // Page break
    doc.push(elements::PageBreak::new());
    
    // --- Sheet 2: Lembar Absensi Harian ---
    doc.push(title_text("CATATAN LEMBAR ABSENSI HARIAN"));
    doc.push(elements::Break::new(1));
    for line in profile_lines {
        doc.push(text(&line));
    }
    doc.push(elements::Break::new(1));
    
    let mut table2 = elements::TableLayout::new(vec![3, 2, 2, 2, 3, 2, 3]);
    table2.set_cell_decorator(elements::FrameCellDecorator::new(true, true, false));
    let _ = table2.push_row(vec![
        boxed(bold_text("Tanggal")),
        boxed(bold_text("Hari")),
        boxed(bold_text("Masuk")),
        boxed(bold_text("Pulang")),
        boxed(bold_text("Total Jam")),
        boxed(bold_text("Paraf")),
        boxed(bold_text("Ket.")),
    ]);
    
    current = first_day;
    while current <= last_day {
        let date_str = current.format("%Y-%m-%d").to_string();
        let is_weekend = matches!(current.weekday(), Weekday::Sat | Weekday::Sun);
        let day_name = crate::day_name_id(current.weekday());
        
        let mut masuk = String::new();
        let mut pulang = String::new();
        let mut total = String::new();
        let paraf = String::new();
        let mut ket = String::new();
        
        if is_weekend { ket = "Libur".to_string(); }
        
        if let Some(log) = log_map.get(&date_str) {
            if let Some(cin) = &log.check_in { masuk = cin.clone(); }
            if let Some(cout) = &log.check_out { pulang = cout.clone(); }
            total = crate::total_hours_str(&masuk, &pulang);
            if let Some(note) = &log.attendance_note { ket = note.clone(); }
        }
        
        let _ = table2.push_row(vec![
            boxed(text(&date_str)),
            boxed(text(day_name)),
            boxed(text(&masuk)),
            boxed(text(&pulang)),
            boxed(text(&total)),
            boxed(text(&paraf)),
            boxed(text(&ket)),
        ]);
        current = current.succ_opt().unwrap();
    }
    doc.push(table2);
    doc.push(elements::Break::new(2));
    
    // TTD Block
    let mut ttd_layout2 = elements::LinearLayout::vertical();
    ttd_layout2.push(right_text(&profile.role));
    ttd_layout2.push(elements::Break::new(3));
    ttd_layout2.push(right_text(&profile.name));
    ttd_layout2.push(right_text(&profile.ni));
    doc.push(ttd_layout2);
    
    // Page break
    doc.push(elements::PageBreak::new());
    
    // --- Sheet 3: Dokumentasi ---
    doc.push(title_text("DOKUMENTASI HARIAN"));
    doc.push(elements::Break::new(2));
    
    for log in logs.iter() {
        if let Some(photo_path) = &log.photo_path {
            if std::path::Path::new(photo_path).exists() {
                doc.push(bold_text(&format!("Tanggal: {}", log.date)));
                doc.push(elements::Break::new(1));
                
                // Add Image
                match elements::Image::from_path(photo_path) {
                    Ok(mut img) => {
                        // constrain image size so it doesn't overflow
                        img.set_alignment(Alignment::Center);
                        doc.push(img);
                    }
                    Err(e) => {
                        doc.push(text(&format!("(Gambar tidak dapat dimuat: {})", e)));
                    }
                }
                
                doc.push(elements::Break::new(2));
            }
        }
    }
    
    doc.render_to_file(filepath)?;

    Ok(())
}
