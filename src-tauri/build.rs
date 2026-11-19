fn main() {
    let windows = tauri_build::WindowsAttributes::new()
        .app_manifest(r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" xmlns:asmv3="urn:schemas-microsoft-com:asm.v3" manifestVersion="1.0">
  <assemblyIdentity name="KruYa" type="win32" version="1.0.0.0"/>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
  <asmv3:trustInfo>
    <asmv3:security>
      <asmv3:requestedPrivileges>
        <asmv3:requestedExecutionLevel level="requireAdministrator" uiAccess="false"/>
      </asmv3:requestedPrivileges>
    </asmv3:security>
  </asmv3:trustInfo>
</assembly>
"#);

    let attrs = tauri_build::Attributes::new().windows_attributes(windows);
    tauri_build::try_build(attrs).expect("Failed to build Tauri application");
}
