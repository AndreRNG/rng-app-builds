fn main() {
    println!("cargo:rerun-if-env-changed=RNG_STUDIO_BUILD");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/studio.ico");
        res.set("ProductName", "RNG Studio");
        res.set("FileDescription", "RNG Studio");
        res.set("CompanyName", "RNG Media S.L.");
        res.compile().expect("windows resources");
    }
}
