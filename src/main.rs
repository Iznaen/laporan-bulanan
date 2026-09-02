use laporan_bulanan::create_ui;
use slint::ComponentHandle;

fn main() -> Result<(), slint::PlatformError> {
    let ui = create_ui()?;
    ui.run()?;
    Ok(())
}
