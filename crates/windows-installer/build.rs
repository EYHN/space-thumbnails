mod build_support;

use std::{env, fs, path::PathBuf, process::Command};

use build_support::{download, run_command, unzip};
use space_thumbnails_windows::constant::PROVIDERS;

use windows::core::{ConstBuffer, GUID};

fn main() {
    if env::var("PROFILE").unwrap() != "release" || cfg!(not(target_os = "windows")) {
        println!("cargo:warning=Windows installer build skipped");
        return;
    }

    let project_dir = env::current_dir()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();

    let assets_dir = env::current_dir().unwrap().join("assets");

    let registy_keys = PROVIDERS.iter().flat_map(|m| m.register("[#MainDLLFile]"));

    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let upgrade_code = GUID::from_signature(ConstBuffer::from_slice(
        format!("Space Thumbnails{}", version).as_bytes(),
    ));

    let mut wix = String::new();
    wix.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    wix.push_str("<Wix xmlns=\"http://schemas.microsoft.com/wix/2006/wi\">\n");
    wix.push_str(&format!("  <Product Id=\"*\" UpgradeCode=\"{:?}\" Version=\"{}\" Language=\"1033\" Name=\"Space Thumbnails\" Manufacturer=\"EYHN\">\n", upgrade_code, version));
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
        "      <Component Id=\"MainDLL\" Guid=\"9cfa17d1-9a2a-40aa-ba6f-57a2adbdc8dc\" Win64=\"yes\">\n",
    );
    wix.push_str(&format!(
        "        <File Id=\"MainDLLFile\" Source=\"{}\" KeyPath=\"yes\" Checksum=\"yes\"/>\n",
        project_dir
            .join("target\\release\\space_thumbnails_windows_dll.dll")
            .to_str()
            .unwrap()
    ));

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
    wix.push_str("      <ComponentRef Id=\"MainDLL\" />\n");
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
    wix.push_str("  </Product>\n");
    wix.push_str("</Wix>\n");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let build_dir = out_dir.join("build");
    fs::create_dir_all(&build_dir).unwrap();

    let installerwxs = build_dir.join("installer.wxs");

    fs::write(&installerwxs, wix).unwrap();

    let wixzip = download(
        out_dir
            .join("download")
            .join("wix311-binaries.zip")
            .to_str()
            .unwrap(),
        "https://github.com/wixtoolset/wix3/releases/download/wix3112rtm/wix311-binaries.zip",
    )
    .unwrap();

    let wixdir = unzip(&wixzip, out_dir.join("wix")).unwrap();

    let mut candle_command = Command::new(wixdir.join("candle.exe"));
    candle_command
        .current_dir(&build_dir)
        .arg(installerwxs.to_str().unwrap())
        .args(["-arch", "x64"]);

    run_command(&mut candle_command, "candle.exe");

    let mut light_command = Command::new(wixdir.join("light.exe"));
    light_command
        .current_dir(&build_dir)
        .arg(build_dir.join("installer.wixobj"))
        .args(["-ext", "WixUIExtension"]);

    run_command(&mut light_command, "light.exe");

    fs::copy(
        build_dir.join("installer.msi"),
        project_dir
            .join("target")
            .join("release")
            .join("space-thumbnails-installer.msi"),
    )
    .unwrap();
}
