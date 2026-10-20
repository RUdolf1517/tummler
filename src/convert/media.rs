use crate::deps;
use crate::error::TummlerError;
use crate::geom::{ffmpeg_scale_filter, Geometry};
use crate::interrupt;
use crate::util::{get_extension, normalize_ext};
use std::path::Path;
use std::process::Command;

/// Map 1..=100 onto x264/x265 CRF 18..=38 (useful range). 100 → 18, 80 → 22, 50 → 28.
fn quality_to_crf(q: u8) -> u8 {
    let q = q.clamp(1, 100);
    (18.0 + f32::from(100 - q) * 0.2).round() as u8
}

fn x264_preset(q: u8) -> &'static str {
    if q >= 80 { "medium" } else { "veryfast" }
}

fn is_video_ext(ext: &str) -> bool {
    matches!(
        ext,
        "mp4" | "avi" | "mkv" | "mov" | "webm" | "flv" | "wmv" | "mpg" | "mpeg" | "3gp"
            | "gif" | "m4v"
    )
}

fn is_audio_ext(ext: &str) -> bool {
    matches!(
        ext,
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "opus" | "wma" | "aiff" | "aif"
            | "alac" | "ac3" | "dts" | "amr" | "au" | "ra"
    )
}

/// Remux (`-c copy`) when only the container changes: no `-q`, no resize, no extra args.
pub fn can_try_stream_copy(
    src_ext: &str,
    dst_ext: &str,
    quality: Option<u8>,
    geometry: &Geometry,
    ffmpeg_args: Option<&str>,
) -> bool {
    if quality.is_some() || !geometry.is_noop() || ffmpeg_args.is_some() {
        return false;
    }
    let src = normalize_ext(src_ext);
    let dst = normalize_ext(dst_ext);
    if src == dst || src == "gif" || dst == "gif" {
        return false;
    }
    (is_video_ext(&src) || is_audio_ext(&src)) && (is_video_ext(&dst) || is_audio_ext(&dst))
}

fn ffmpeg_copy(input: &Path, output: &Path) -> bool {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"]);
    cmd.arg("-i").arg(input);
    cmd.args(["-c", "copy"]);
    cmd.arg(output);
    interrupt::run(&mut cmd)
        .ok()
        .is_some_and(|r| r.status.success())
}

/// Audio/video conversion via ffmpeg.
#[allow(clippy::too_many_arguments)]
pub fn convert_media(
    input: &Path,
    output: &Path,
    quality: Option<u8>,
    ffmpeg_args: Option<&str>,
    no_auto_install: bool,
    assume_yes: bool,
    geometry: &Geometry,
    ffmpeg_threads: usize,
) -> Result<(), TummlerError> {
    deps::ensure_available("ffmpeg", no_auto_install, assume_yes)
        .map_err(|e| TummlerError::DependencyMissing("ffmpeg".into(), e))?;

    let in_ext = get_extension(input).unwrap_or_default();
    let out_ext = output
        .extension()
        .and_then(|e| e.to_str())
        .map(normalize_ext)
        .unwrap_or_default();
    if can_try_stream_copy(&in_ext, &out_ext, quality, geometry, ffmpeg_args)
        && ffmpeg_copy(input, output)
    {
        return Ok(());
    }

    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"]);
    cmd.arg("-i");
    cmd.arg(input);

    let user_has = |flag: &str| {
        ffmpeg_args.is_some_and(|s| s.split_whitespace().any(|a| a == flag))
    };
    if ffmpeg_threads > 0 && !user_has("-threads") {
        cmd.arg("-threads").arg(ffmpeg_threads.to_string());
    }

    let user_has_vf = user_has("-vf") || user_has("-filter:v") || user_has("-filter_complex");
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
            if is_video_ext(&out_ext) && out_ext != "gif" {
                let crf = quality_to_crf(q);
                cmd.arg("-crf").arg(crf.to_string());
                if out_ext == "webm" && !user_has("-b:v") {
                    // VP9 CRF is ignored unless bitrate is unconstrained.
                    cmd.args(["-b:v", "0"]);
                }
                // -preset is libx264/x265 only; webm/vp9 fails on it.
                if matches!(out_ext.as_str(), "mp4" | "mkv" | "mov" | "m4v") {
                    cmd.arg("-preset").arg(x264_preset(q));
                }
            } else if is_audio_ext(&out_ext) {
                let bitrate = match q {
                    90..=100 => "320k",
                    80..=89 => "256k",
                    60..=79 => "192k",
                    40..=59 => "128k",
                    20..=39 => "96k",
                    _ => "64k",
                };
                cmd.arg("-b:a").arg(bitrate);
            } else {
                // Image via ffmpeg (lossy webp, heic, …). libwebp: 0–100 higher=better.
                cmd.arg("-q:v").arg(q.to_string());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Geometry;

    #[test]
    fn crf_stays_in_useful_range() {
        assert_eq!(quality_to_crf(100), 18);
        assert_eq!(quality_to_crf(90), 20);
        assert_eq!(quality_to_crf(80), 22);
        assert_eq!(quality_to_crf(50), 28);
        assert_eq!(quality_to_crf(1), 38);
        assert!(quality_to_crf(90) > quality_to_crf(100));
    }

    #[test]
    fn preset_never_slow() {
        assert_eq!(x264_preset(100), "medium");
        assert_eq!(x264_preset(80), "medium");
        assert_eq!(x264_preset(79), "veryfast");
    }

    #[test]
    fn stream_copy_only_for_container_change() {
        let g = Geometry::default();
        assert!(can_try_stream_copy("mkv", "mp4", None, &g, None));
        assert!(can_try_stream_copy("mp4", "mkv", None, &g, None));
        assert!(can_try_stream_copy("m4a", "mp4", None, &g, None));
        assert!(!can_try_stream_copy("mkv", "mp4", Some(80), &g, None));
        assert!(!can_try_stream_copy("mkv", "gif", None, &g, None));
        assert!(!can_try_stream_copy("mp4", "mp4", None, &g, None));
        assert!(!can_try_stream_copy("png", "mp4", None, &g, None));
        let resized = Geometry {
            max: Some(1920),
            resize: None,
        };
        assert!(!can_try_stream_copy("mkv", "mp4", None, &resized, None));
        assert!(!can_try_stream_copy(
            "mkv",
            "mp4",
            None,
            &g,
            Some("-an")
        ));
    }
}
