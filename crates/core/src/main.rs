use std::error::Error;

mod studio;

fn main() -> Result<(), Box<dyn Error>> {
    let mut desktop_mode = false;
    let mut widget_mode = false;
    for argument in std::env::args().skip(1) {
        match argument.as_str() {
            "--desktop" => {
                desktop_mode = true;
                widget_mode = true;
            }
            "--widget" => widget_mode = true,
            "--help" | "-h" => {
                println!("Usage: glassdesk-core [--widget | --desktop]");
                return Ok(());
            }
            _ => return Err(format!("Unknown argument: {argument}").into()),
        }
    }

    if widget_mode {
        #[cfg(not(windows))]
        return Err("GlassDesk widgets currently require Windows 11.".into());

        #[cfg(windows)]
        glassdesk_widget_host::run(desktop_mode)?;
    } else {
        studio::run()?;
    }

    Ok(())
}
