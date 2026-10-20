use crate::deps;
use crate::error::TummlerError;
use std::path::Path;
use std::process::Command;

/// Конвертация аудио/видео через ffmpeg
pub fn convert_media(
    input: &Path,
    output: &Path,
    quality: Option<u8>,
    ffmpeg_args: Option<&str>,
    no_auto_install: bool,
) -> Result<(), TummlerError> {
    // Проверяем ffmpeg
    deps::ensure_available("ffmpeg", no_auto_install)
        .map_err(|e| TummlerError::DependencyMissing("ffmpeg".into(), e))?;

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-i").arg(input);
    cmd.arg("-y"); // overwrite (мы уже проверили --overwrite, но ffmpeg должен перезаписать)

    // Качество
    if let Some(q) = quality {
        let q = q.clamp(1, 100);
        // Для видео: crf — обратная шкала, 0 лучшее, 51 худшее
        // Формула: crf = (100 - q) * 51 / 100
        // q=90 → crf=5, q=80 → crf=10, q=50 → crf=25
        let crf = ((100 - q) as f32 * 51.0 / 100.0).round() as u8;
        // Применяем только для видеоформатов
        let out_ext = output
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let is_video = matches!(
            out_ext.as_str(),
            "mp4" | "avi" | "mkv" | "mov" | "webm" | "flv" | "wmv" | "mpg" | "mpeg" | "3gp" | "gif" | "m4v"
        );
        if is_video {
            // Для gif — особые настройки
            if out_ext == "gif" {
                // gif не использует crf, но можно scale
                // оставляем дефолт ffmpeg
            } else {
                cmd.arg("-crf").arg(crf.to_string());
                // preset для баланса скорости/качества
                if q >= 80 {
                    cmd.arg("-preset").arg("slow");
                } else {
                    cmd.arg("-preset").arg("medium");
                }
            }
        } else {
            // Аудио: битрейт
            // q 90 → 320k, q 50 → 128k, q 10 → 32k
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

    // Дополнительные аргументы пользователя
    if let Some(extra) = ffmpeg_args {
        for arg in shell_words::split(extra).unwrap_or_else(|_| extra.split_whitespace().map(|s| s.to_string()).collect()) {
            cmd.arg(arg);
        }
    }

    cmd.arg(output);

    let output_result = cmd.output().map_err(|e| TummlerError::MediaError(e.to_string()))?;

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

// Простой парсер shell-аргументов без зависимости shell-words (фолбэк)
// Если shell-words не доступен, используем split_whitespace
// Добавим опциональную зависимость если нужно, но пока делаем просто
mod shell_words {
    pub fn split(input: &str) -> Result<Vec<String>, String> {
        // Минимальный парсер с поддержкой кавычек
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
                        result.push(current.clone());
                        current.clear();
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
}
