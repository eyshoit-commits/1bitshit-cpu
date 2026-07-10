fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set("InternalName", "cluaiz.exe");
        res.set("FileDescription", "Cluaiz AI Engine");
        res.set("ProductName", "Cluaiz");
        res.set("OriginalFilename", "cluaiz.exe");
        res.set("LegalCopyright", "Copyright © 2026 Cluaiz.");
        res.set("CompanyName", "Cluaiz");
        
        res.set_manifest(r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
<trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
        <requestedPrivileges>
            <requestedExecutionLevel level="asInvoker" uiAccess="false" />
        </requestedPrivileges>
    </security>
</trustInfo>
</assembly>
"#);
        
        // 🚀 Set the Cluaiz Taskbar & Executable Icon
        res.set_icon("../../assets/logo.ico");
        
        if let Err(e) = res.compile() {
            eprintln!("Failed to compile Windows resources: {}", e);
        }
    }
}
  