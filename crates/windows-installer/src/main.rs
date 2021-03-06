mod build_support;

use std::{env, fs, path::PathBuf, process::Command};

use build_support::{download, run_command, unzip};
use space_thumbnails_windows::constant::PROVIDERS;

fn main() {
    let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();

    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    let out_dir = project_dir.join("target").join("installer");
    let download_dir = out_dir.join("download");
    fs::create_dir_all(download_dir).unwrap();

    let build_dir = out_dir.join("build");
    fs::create_dir_all(&build_dir).unwrap();

    let registy_keys = PROVIDERS.iter().flat_map(|m| m.register("[#MainDLLFile]"));

    let version = env!("CARGO_PKG_VERSION");

    let mut wix = String::new();
    wix.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    wix.push_str("<Wix xmlns=\"http://schemas.microsoft.com/wix/2006/wi\" xmlns:util=\"http://schemas.microsoft.com/wix/UtilExtension\">\n");
    wix.push_str(&format!("  <Product Id=\"*\" UpgradeCode=\"1C589985-B4C6-53EC-8483-112D02E6DCD2\" Version=\"{}\" Language=\"1033\" Name=\"Space Thumbnails\" Manufacturer=\"EYHN\">\n", version));
    wix.push_str(
        "    <Package InstallerVersion=\"300\" Compressed=\"yes\" InstallScope=\"perMachine\"/>\n",
    );
    wix.push_str("    <Media Id=\"1\" Cabinet=\"cab1.cab\" EmbedCab=\"yes\" />\n");
    wix.push_str("    <Directory Id=\"TARGETDIR\" Name=\"SourceDir\">\n");
    wix.push_str("      <Directory Id=\"ProgramFiles64Folder\">\n");
    wix.push_str(
        "        <Directory Id=\"APPLICATIONROOTDIRECTORY\" Name=\"Space Thumbnails\"/>\n",
    );
    wix.push_str("      </Directory>\n");
    wix.push_str("    </Directory>\n");

    wix.push_str("    <DirectoryRef Id=\"APPLICATIONROOTDIRECTORY\">\n");
    wix.push_str(
        "      <Component Id=\"MainApplication\" Guid=\"9cfa17d1-9a2a-40aa-ba6f-57a2adbdc8dc\" Win64=\"yes\">\n",
    );
    wix.push_str(&format!(
        "        <File Id=\"MainDLLFile\" Source=\"{}\" KeyPath=\"yes\" Checksum=\"yes\"/>\n",
        project_dir
            .join("target\\release\\space_thumbnails_windows_dll.dll")
            .to_str()
            .unwrap()
    ));
    wix.push_str(&format!(
        "        <File Id=\"LicenceFile\" Source=\"{}\" Checksum=\"yes\"/>\n",
        assets_dir.join("Licence.rtf").to_str().unwrap()
    ));
    wix.push_str(&format!(
        "        <File Id=\"ReadmeFile\" Source=\"{}\" Checksum=\"yes\"/>\n",
        project_dir.join("README.md").to_str().unwrap()
    ));
    wix.push_str("        <util:EventSource EventMessageFile=\"[#MainDLLFile]\" Log=\"Application\" Name=\"Space Thumbnails\"/>\n");

    for key in registy_keys {
        wix.push_str(&format!(
            "        <RegistryKey Root=\"HKCR\" Key=\"{}\">\n",
            &key.path
        ));
        for val in key.values {
            let (val_type, val_data) = match val.1 {
                space_thumbnails_windows::registry::RegistryData::Str(data) => ("string", data),
                space_thumbnails_windows::registry::RegistryData::U32(data) => {
                    ("integer", data.to_string())
                }
            };

            if val.0.is_empty() {
                wix.push_str(&format!(
                    "            <RegistryValue Type=\"{}\" Value=\"{}\"/>\n",
                    val_type, val_data
                ));
            } else {
                wix.push_str(&format!(
                    "            <RegistryValue Type=\"{}\" Name=\"{}\" Value=\"{}\"/>\n",
                    val_type, val.0, val_data
                ));
            }
        }
        wix.push_str("        </RegistryKey>\n");
    }

    wix.push_str("      </Component>\n");
    wix.push_str("    </DirectoryRef>\n");

    wix.push_str("    <Feature Id=\"MainFeature\" Title=\"Space Thumbnails\" Level=\"1\">\n");
    wix.push_str("      <ComponentRef Id=\"MainApplication\" />\n");
    wix.push_str("    </Feature>\n");
    wix.push_str("    <UIRef Id=\"WixUI_Minimal\" />\n");
    wix.push_str("    <UIRef Id=\"WixUI_ErrorProgressText\" />\n");
    wix.push_str(&format!(
        "    <Icon Id=\"icon.ico\" SourceFile=\"{}\"/>\n",
        assets_dir.join("icon.ico").to_str().unwrap()
    ));
    wix.push_str("    <Property Id=\"ARPPRODUCTICON\" Value=\"icon.ico\" />\n");
    wix.push_str(&format!(
        "    <WixVariable Id=\"WixUIDialogBmp\" Value=\"{}\" />\n",
        assets_dir.join("UIDialog.bmp").to_str().unwrap()
    ));
    wix.push_str(&format!(
        "    <WixVariable Id=\"WixUIBannerBmp\" Value=\"{}\" />\n",
        assets_dir.join("UIBanner.bmp").to_str().unwrap()
    ));
    wix.push_str(&format!(
        "    <WixVariable Id=\"WixUILicenseRtf\" Value=\"{}\" />\n",
        assets_dir.join("Licence.rtf").to_str().unwrap()
    ));
    wix.push_str("    <MajorUpgrade AllowDowngrades=\"no\" AllowSameVersionUpgrades=\"no\" DowngradeErrorMessage=\"A newer version of [ProductName] is already installed.  If you are sure you want to downgrade, remove the existing installation via the Control Panel\" />\n");
    wix.push_str("  </Product>\n");
    wix.push_str("</Wix>\n");

    let installerwxs = build_dir.join("installer.wxs");

    fs::write(&installerwxs, wix).unwrap();

    let wixzip = download(
        out_dir.join("download").join("wix311-binaries.zip"),
        "https://github.com/wixtoolset/wix3/releases/download/wix3112rtm/wix311-binaries.zip",
    )
    .unwrap();

    let wixdir = unzip(&wixzip, out_dir.join("wix")).unwrap();

    let mut candle_command = Command::new(wixdir.join("candle.exe"));
    candle_command
        .current_dir(&build_dir)
        .arg(installerwxs.to_str().unwrap())
        .args(["-arch", "x64"])
        .args(["-ext", "WixUtilExtension"]);

    run_command(&mut candle_command, "candle.exe");

    let mut light_command = Command::new(wixdir.join("light.exe"));
    light_command
        .current_dir(&build_dir)
        .arg(build_dir.join("installer.wixobj"))
        .args(["-ext", "WixUIExtension"])
        .args(["-ext", "WixUtilExtension"]);

    run_command(&mut light_command, "light.exe");

    fs::copy(
        build_dir.join("installer.msi"),
        out_dir.join("space-thumbnails-installer.msi"),
    )
    .unwrap();
}
