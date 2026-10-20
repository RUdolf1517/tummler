use crate::deps;
use crate::error::TummlerError;
use crate::geom::{ffmpeg_scale_filter, Geometry};
use crate::interrupt;
use crate::util::normalize_ext;
use std::path::Path;
use std::process::Command;

/// Audio/video conversion via ffmpeg
pub fn convert_media(
    input: &Path,
    output: &Path,
    quality: Option<u8>,
    ffmpeg_args: Option<&str>,
    no_auto_install: bool,
    assume_yes: bool,
    geometry: &Geometry,
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

    let user_has_vf = ffmpeg_args.is_some_and(|s| {
        s.split_whitespace()
            .any(|a| a == "-vf" || a == "-filter:v" || a == "-filter_complex")
    });
    let scale = if user_has_vf {
        None
    } else {
        ffmpeg_scale_filter(geometry)
    };

    if out_ext == "gif" && ffmpeg_args.is_none() {
        let scale_part = scale
            .as_deref()
            .unwrap_or("scale=480:-1:flags=lanczos");
        let vf = format!(
            "fps=15,{scale_part},split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse"
        );
        cmd.args(["-vf", &vf, "-loop", "0"]);
    } else {
        if let Some(ref vf) = scale {
            cmd.args(["-vf", vf]);
        }
        if let Some(q) = quality {
            let q = q.clamp(1, 100);
            let is_video = matches!(
                out_ext.as_str(),
                "mp4" | "avi" | "mkv" | "mov" | "webm" | "flv" | "wmv" | "mpg" | "mpeg" | "3gp"
                    | "gif" | "m4v"
            );
            if is_video && out_ext != "gif" {
                let crf = ((100 - q) as f32 * 51.0 / 100.0).round() as u8;
                cmd.arg("-crf").arg(crf.to_string());
                // -preset is libx264/x265 only; webm/vp9 does not have it and ffmpeg fails
                if matches!(out_ext.as_str(), "mp4" | "mkv" | "mov" | "m4v") {
                    cmd.arg("-preset").arg(if q >= 80 { "slow" } else { "medium" });
                }
            } else if !is_video {
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

    let output_result = interrupt::run(&mut cmd)
        .map_err(|e| TummlerError::MediaError(e.to_string()))?;

    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        let stdout = String::from_utf8_lossy(&output_result.stdout);
        return Err(TummlerError::MediaError(format!(
            "ffmpeg failed (exit {:?}):\n{}\n{}",
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
            return Err("unclosed quote in --ffmpeg-args".into());
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
