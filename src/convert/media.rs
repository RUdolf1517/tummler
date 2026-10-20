use crate::deps;
use crate::error::TummlerError;
use crate::util::normalize_ext;
use std::path::Path;
use std::process::Command;

/// Конвертация аудио/видео через ffmpeg
pub fn convert_media(
    input: &Path,
    output: &Path,
    quality: Option<u8>,
    ffmpeg_args: Option<&str>,
    no_auto_install: bool,
    assume_yes: bool,
) -> Result<(), TummlerError> {
    deps::ensure_available("ffmpeg", no_auto_install, assume_yes)
        .map_err(|e| TummlerError::DependencyMissing("ffmpeg".into(), e))?;

    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-i"]);
    cmd.arg(input);

    let out_ext = output
        .extension()
        .and_then(|e| e.to_str())
        .map(normalize_ext)
        .unwrap_or_default();

    if out_ext == "gif" && ffmpeg_args.is_none() {
        cmd.args([
            "-vf",
            "fps=15,scale=480:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse",
            "-loop",
            "0",
        ]);
    } else if let Some(q) = quality {
        let q = q.clamp(1, 100);
        let is_video = matches!(
            out_ext.as_str(),
            "mp4" | "avi" | "mkv" | "mov" | "webm" | "flv" | "wmv" | "mpg" | "mpeg" | "3gp"
                | "gif" | "m4v"
        );
        if is_video {
            if out_ext != "gif" {
                let crf = ((100 - q) as f32 * 51.0 / 100.0).round() as u8;
                cmd.arg("-crf").arg(crf.to_string());
                if q >= 80 {
                    cmd.arg("-preset").arg("slow");
                } else {
                    cmd.arg("-preset").arg("medium");
                }
            }
        } else {
            let bitrate = match q {
                90..=100 => "320k",
                80..=89 => "256k",
                60..=79 => "192k",
                40..=59 => "128k",
                20..=39 => "96k",
                _ => "64k",
            };
            cmd.arg("-b:a").arg(bitrate);
        }
    }

    if let Some(extra) = ffmpeg_args {
        match shell_words::split(extra) {
            Ok(args) => {
                for arg in args {
                    cmd.arg(arg);
                }
            }
            Err(e) => return Err(TummlerError::MediaError(e)),
        }
    }

    cmd.arg(output);

    let output_result = cmd
        .output()
        .map_err(|e| TummlerError::MediaError(e.to_string()))?;

    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        let stdout = String::from_utf8_lossy(&output_result.stdout);
        return Err(TummlerError::MediaError(format!(
            "ffmpeg завершился с ошибкой (код {:?}):\n{}\n{}",
            output_result.status.code(),
            stderr,
            stdout
        )));
    }

    Ok(())
}

mod shell_words {
    pub fn split(input: &str) -> Result<Vec<String>, String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut in_single = false;
        let mut in_double = false;
        let mut escaped = false;

        for c in input.chars() {
            if escaped {
                current.push(c);
                escaped = false;
                continue;
            }
            match c {
                '\\' if !in_single => {
                    escaped = true;
                }
                '\'' if !in_double => {
                    in_single = !in_single;
                }
                '"' if !in_single => {
                    in_double = !in_double;
                }
                ' ' | '\t' | '\n' if !in_single && !in_double => {
                    if !current.is_empty() {
                        result.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(c),
            }
        }
        if in_single || in_double {
            return Err("Незакрытая кавычка в --ffmpeg-args".into());
        }
        if !current.is_empty() {
            result.push(current);
        }
        Ok(result)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn splits_plain() {
            assert_eq!(
                split("-crf 23 -preset fast").unwrap(),
                vec!["-crf", "23", "-preset", "fast"]
            );
        }

        #[test]
        fn splits_quotes() {
            assert_eq!(
                split(r#"-vf "scale=1280:720" -an"#).unwrap(),
                vec!["-vf", "scale=1280:720", "-an"]
            );
        }

        #[test]
        fn rejects_unclosed() {
            assert!(split(r#"-vf "scale"#).is_err());
        }
    }
}
