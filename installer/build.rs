fn main() {
    let url = std::env::var("POCKETOS_MANIFEST_URL").unwrap_or_else(|_| {
        "https://raw.githubusercontent.com/OWNER/PocketOS/usb-beta/manifest.json".into()
    });
    println!("cargo:rustc-env=POCKETOS_MANIFEST_URL={url}");
    println!("cargo:rerun-if-env-changed=POCKETOS_MANIFEST_URL");
}
