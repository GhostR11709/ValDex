fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=../../vivi.ico");
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../vivi.ico");
        res.set("ProductName", "ValDex");
        res.set("FileDescription", "ValDex");
        res.set("InternalName", "ValDex");
        res.set("OriginalFilename", "vivi-start.exe");
        res.compile().expect("the ValDex icon did not embed");
    }
}
