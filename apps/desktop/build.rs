fn main() {
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-changed=assets/piutang-reza-putra.ico");
    #[cfg(windows)]
    embed_resource::compile("assets/app.rc", embed_resource::NONE)
        .manifest_optional()
        .expect("failed to embed Windows application icon");
}
