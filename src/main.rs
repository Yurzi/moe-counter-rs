use std::io::Write;

mod banner;
mod utils;

fn main() {
    let theme_manager = banner::ThemeManager::new("themes").expect("has error");
    let image = theme_manager
        .get("asoul")
        .unwrap()
        .gen_webp(114514, 7)
        .unwrap();

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    handle.write_all(&image.encode().unwrap());
    handle.flush();
}
