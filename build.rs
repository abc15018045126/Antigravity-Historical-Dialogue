#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("icon.ico");
    if let Err(e) = res.compile() {
        eprintln!("Warning: Failed to compile Windows resource icon: {e}");
    }
}

#[cfg(not(windows))]
fn main() {}
