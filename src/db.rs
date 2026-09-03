use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct UserProfile {
    pub id: i32,
    pub name: String,
    pub ni: String,
    pub role: String,
    pub work_unit: String,
    pub signature_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DailyLog {
    pub date: String, // Format: YYYY-MM-DD
    pub check_in: Option<String>, // Format: HH:MM
    pub check_out: Option<String>,
    pub attendance_note: Option<String>,
    pub activity_desc: Option<String>,
    pub activity_output: Option<String>,
    pub activity_note: Option<String>,
    pub photo_path: Option<String>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Opens or creates a database at the specified file path.
    pub fn new<P: AsRef<Path>>(path: P) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let db = Database { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Initializes the database schema if it doesn't exist.
    fn init_schema(&self) -> SqlResult<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS user_profile (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                ni TEXT NOT NULL,
                role TEXT NOT NULL,
                work_unit TEXT NOT NULL,
                signature_path TEXT
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS daily_logs (
                date TEXT PRIMARY KEY,
                check_in TEXT,
                check_out TEXT,
                attendance_note TEXT,
                activity_desc TEXT,
                activity_output TEXT,
                activity_note TEXT,
                photo_path TEXT
            )",
            [],
        )?;

        Ok(())
    }

    /// Saves or updates the single user profile. We assume id=1 for the primary user.
    pub fn save_profile(&self, profile: &UserProfile) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO user_profile (id, name, ni, role, work_unit, signature_path)
             VALUES (1, ?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                ni = excluded.ni,
                role = excluded.role,
                work_unit = excluded.work_unit,
                signature_path = excluded.signature_path",
            params![
                profile.name,
                profile.ni,
                profile.role,
                profile.work_unit,
                profile.signature_path
            ],
        )?;
        Ok(())
    }

    /// Retrieves the user profile if it exists.
    pub fn get_profile(&self) -> SqlResult<Option<UserProfile>> {
        self.conn.query_row(
            "SELECT id, name, ni, role, work_unit, signature_path FROM user_profile WHERE id = 1",
            [],
            |row| {
                Ok(UserProfile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    ni: row.get(2)?,
                    role: row.get(3)?,
                    work_unit: row.get(4)?,
                    signature_path: row.get(5)?,
                })
            },
        ).optional()
    }

    /// Saves or updates a daily log entry based on its date.
    pub fn save_daily_log(&self, log: &DailyLog) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO daily_logs (
                date, check_in, check_out, attendance_note,
                activity_desc, activity_output, activity_note, photo_path
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(date) DO UPDATE SET
                check_in = excluded.check_in,
                check_out = excluded.check_out,
                attendance_note = excluded.attendance_note,
                activity_desc = excluded.activity_desc,
                activity_output = excluded.activity_output,
                activity_note = excluded.activity_note,
                photo_path = excluded.photo_path",
            params![
                log.date, log.check_in, log.check_out, log.attendance_note,
                log.activity_desc, log.activity_output, log.activity_note, log.photo_path
            ],
        )?;
        Ok(())
    }

    /// Retrieves a daily log entry by date (YYYY-MM-DD).
    pub fn get_daily_log(&self, date: &str) -> SqlResult<Option<DailyLog>> {
        self.conn.query_row(
            "SELECT date, check_in, check_out, attendance_note, 
                    activity_desc, activity_output, activity_note, photo_path 
             FROM daily_logs WHERE date = ?1",
            params![date],
            |row| {
                Ok(DailyLog {
                    date: row.get(0)?,
                    check_in: row.get(1)?,
                    check_out: row.get(2)?,
                    attendance_note: row.get(3)?,
                    activity_desc: row.get(4)?,
                    activity_output: row.get(5)?,
                    activity_note: row.get(6)?,
                    photo_path: row.get(7)?,
                })
            },
        ).optional()
    }

    /// Retrieves all daily logs, ordered by date.
    pub fn get_all_daily_logs(&self) -> SqlResult<Vec<DailyLog>> {
        let mut stmt = self.conn.prepare(
            "SELECT date, check_in, check_out, attendance_note, 
                    activity_desc, activity_output, activity_note, photo_path 
             FROM daily_logs ORDER BY date ASC"
        )?;
        let log_iter = stmt.query_map([], |row| {
            Ok(DailyLog {
                date: row.get(0)?,
                check_in: row.get(1)?,
                check_out: row.get(2)?,
                attendance_note: row.get(3)?,
                activity_desc: row.get(4)?,
                activity_output: row.get(5)?,
                activity_note: row.get(6)?,
                photo_path: row.get(7)?,
            })
        })?;

        let mut logs = Vec::new();
        for log in log_iter {
            logs.push(log?);
        }
        Ok(logs)
    }

    /// Retrieves all daily logs for a given month, ordered by date.
    /// `year_month` must be in "YYYY-MM" format (e.g. "2026-09").
    pub fn get_logs_for_month(&self, year_month: &str) -> SqlResult<Vec<DailyLog>> {
        let mut stmt = self.conn.prepare(
            "SELECT date, check_in, check_out, attendance_note,
                    activity_desc, activity_output, activity_note, photo_path
             FROM daily_logs WHERE date LIKE ?1 ORDER BY date ASC",
        )?;
        let pattern = format!("{}%", year_month);
        let log_iter = stmt.query_map([pattern.as_str()], |row| {
            Ok(DailyLog {
                date: row.get(0)?,
                check_in: row.get(1)?,
                check_out: row.get(2)?,
                attendance_note: row.get(3)?,
                activity_desc: row.get(4)?,
                activity_output: row.get(5)?,
                activity_note: row.get(6)?,
                photo_path: row.get(7)?,
            })
        })?;

        let mut logs = Vec::new();
        for log in log_iter {
            logs.push(log?);
        }
        Ok(logs)
    }
}
