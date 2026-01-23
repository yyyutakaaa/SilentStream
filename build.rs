use std::io;

#[cfg(windows)]
fn main() -> io::Result<()> {
    let mut res = winres::WindowsResource::new();
    res.set_icon("app_icon.ico");
    res.compile()?;
    Ok(())
}

#[cfg(not(windows))]
fn main() {
}
