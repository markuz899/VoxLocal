fn main() {
    // whisper.cpp con Metal usa `@available`, cioè ___isPlatformVersionAtLeast del runtime di
    // clang. Con Xcode 16 (runner di GitHub) rustc non lo collega più da solo: lo si chiede qui.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let dir = std::process::Command::new("clang")
            .arg("--print-resource-dir")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok());
        if let Some(dir) = dir {
            println!("cargo:rustc-link-search=native={}/lib/darwin", dir.trim());
            println!("cargo:rustc-link-lib=static=clang_rt.osx");
        }
    }
    tauri_build::build()
}
