use std::path::PathBuf;

pub(crate) fn find_cjk_fonts() -> Option<Vec<PathBuf>> {
    #[cfg(unix)]
    {
        use std::process::Command;
        // linux/macOS command: fc-list
        let output = Command::new("sh").arg("-c").arg("fc-list").output().ok()?;
        let stdout = std::str::from_utf8(&output.stdout).ok()?;
        #[cfg(target_os = "macos")]
        {
            // Japanese, Korean, Chinese
            static FONT_FILES: [(&str, &str); 3] = [
                ("明朝", "/System/Library/Fonts/ヒラギノ明朝 ProN.ttc"),
                ("AppleGothic", "/System/Library/Fonts/Supplemental/AppleGothic.ttf"),
                ("Hiragino Sans GB", "/System/Library/Fonts/Hiragino Sans GB.ttc"),
            ];
            let font_files = FONT_FILES
                .iter()
                .filter_map(|(font_name, def_font_path)| {
                    let font_line = stdout
                        .lines()
                        .find(|line| line.contains("Regular") && line.contains(font_name))
                        .unwrap_or(def_font_path);
                    let font_path = font_line.split(':').next()?.trim();
                    let font_path = PathBuf::from(font_path);
                    if font_path.exists() {
                        Some(font_path)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            Some(font_files)
        }
        #[cfg(target_os = "linux")]
        {
            let def_cjk_font = "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc";
            let font_line = stdout
                .lines()
                .find(|line| line.contains("Regular") && line.contains("CJK"))
                .unwrap_or(def_cjk_font);
            let font_path = font_line.split(':').next()?.trim();
            let font_path = PathBuf::from(font_path);
            if !font_path.exists() {
                return None;
            }
            Some(vec![font_path])
        }
    }
    #[cfg(windows)]
    {
        // Japanese c:/Windows/Fonts/msgothic.ttc
        // Korean   c:/Windows/Fonts/malgun.ttf
        // Chinese  c:/Windows/Fonts/msyh.ttc
        static FONT_FILES: [&str; 3] = ["msgothic.ttc", "malgun.ttf", "msyh.ttc"];

        let mut font_path = PathBuf::from(std::env::var("SystemRoot").ok()?);
        font_path.push("Fonts");
        let font_files = FONT_FILES
            .iter()
            .map(|font_file| font_path.join(font_file))
            .filter(|font_file| font_file.exists())
            .collect::<Vec<_>>();
        Some(font_files)
    }
}

#[test]
fn test_find_cjk_fonts() {
    let font_files = find_cjk_fonts();
    assert!(font_files.unwrap().len() > 0);
}
