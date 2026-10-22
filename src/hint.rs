/// One-line hint for known engine logs. Raw stderr/stdout stay; this is appended.
pub fn hint_for(log: &str) -> Option<&'static str> {
    if log.is_empty() {
        return None;
    }
    let s = log.to_ascii_lowercase();

    if s.contains("pdflatex") || s.contains("xelatex") || s.contains("lualatex") {
        return Some(
            "PDF needs LaTeX (e.g. brew install --cask mactex / sudo apt install texlive-latex-recommended texlive-fonts-recommended)",
        );
    }
    if s.contains("unknown encoder")
        || s.contains("encoder not found")
        || s.contains("unknown decoder")
        || s.contains("decoder not found")
    {
        return Some("this ffmpeg build lacks that codec (try another format, or a fuller ffmpeg)");
    }
    if s.contains("invalid data found when processing input")
        || s.contains("moov atom not found")
        || s.contains("does not contain any stream")
    {
        return Some("input looks corrupt, empty, or not the format the name/sniff says");
    }
    if s.contains("no space left on device") {
        return Some("disk full");
    }
    if s.contains("may not be a pdf file")
        || s.contains("pdf file is damaged")
        || s.contains("incorrect password")
    {
        return Some("not a PDF, damaged, or password-protected");
    }
    if s.contains("unknown writer")
        || s.contains("unknown reader")
        || s.contains("unknown output format")
    {
        return Some("tummler --list-formats for supported formats");
    }
    None
}

/// `{name} failed (exit …):` + logs + optional `hint:` line.
pub fn format_engine_fail(
    name: &str,
    code: Option<i32>,
    stderr: &str,
    stdout: &str,
) -> String {
    let mut s = format!("{name} failed (exit {code:?}):\n{stderr}");
    if !stdout.is_empty() {
        if !stderr.is_empty() && !s.ends_with('\n') {
            s.push('\n');
        }
        s.push_str(stdout);
    }
    if let Some(h) = hint_for(stderr).or_else(|| hint_for(stdout)) {
        if !s.ends_with('\n') {
            s.push('\n');
        }
        s.push_str("hint: ");
        s.push_str(h);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latex_hint_matches_existing_wording() {
        let h = hint_for("pdflatex not found").unwrap();
        assert!(h.contains("mactex"));
        assert!(h.contains("texlive-latex-recommended"));
        assert_eq!(hint_for("xelatex: command not found").unwrap(), h);
        assert_eq!(hint_for("lualatex failed").unwrap(), h);
    }

    #[test]
    fn ffmpeg_codec_and_corrupt() {
        assert!(hint_for("Unknown encoder 'libx264'")
            .unwrap()
            .contains("codec"));
        assert!(hint_for("Encoder not found: libwebp")
            .unwrap()
            .contains("codec"));
        assert!(hint_for("Invalid data found when processing input")
            .unwrap()
            .contains("corrupt"));
        assert!(hint_for("moov atom not found").unwrap().contains("corrupt"));
    }

    #[test]
    fn poppler_and_pandoc() {
        assert!(hint_for("Syntax Error: May not be a PDF file")
            .unwrap()
            .contains("PDF"));
        assert!(hint_for("Command Line Error: Incorrect password")
            .unwrap()
            .contains("password"));
        assert!(hint_for("Unknown writer: xyz")
            .unwrap()
            .contains("--list-formats"));
    }

    #[test]
    fn unknown_log_has_no_hint() {
        assert!(hint_for("").is_none());
        assert!(hint_for("frame=  12 fps=0.0").is_none());
        assert!(hint_for("Permission denied").is_none());
    }

    #[test]
    fn format_keeps_stderr_then_hint() {
        let msg = format_engine_fail(
            "pandoc",
            Some(47),
            "pdflatex not found\nmore detail\n",
            "",
        );
        assert!(msg.starts_with("pandoc failed (exit Some(47)):"));
        assert!(msg.contains("pdflatex not found"));
        assert!(msg.contains("more detail"));
        let hint_at = msg.find("hint: ").unwrap();
        let latex_at = msg.find("pdflatex not found").unwrap();
        assert!(latex_at < hint_at);
        assert!(msg.contains("mactex"));
    }
}
