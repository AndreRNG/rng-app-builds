fn main() {
    println!("cargo:rerun-if-env-changed=LAUNCHLLAMA_BUILD");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/launchllama.ico");
        res.set("ProductName", "LaunchLlama");
        res.set("FileDescription", "LaunchLlama");
        res.set("CompanyName", "RNG Media S.L.");
        res.compile().expect("windows resources");
    }
}
