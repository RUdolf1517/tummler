/// Resize / max-side geometry for images and video.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Geometry {
    pub max: Option<u32>,
    pub resize: Option<ResizeSpec>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResizeSpec {
    Percent(f32),
    Width(u32),
    Height(u32),
    Fit(u32, u32),
}

impl Geometry {
    pub fn is_noop(&self) -> bool {
        self.max.is_none() && self.resize.is_none()
    }

    /// New size, aspect ratio kept. `--max` never upscales.
    pub fn target_size(&self, w: u32, h: u32) -> (u32, u32) {
        let mut nw = w.max(1);
        let mut nh = h.max(1);
        if let Some(r) = &self.resize {
            match *r {
                ResizeSpec::Percent(p) => {
                    let s = (p / 100.0).max(0.0);
                    nw = ((w as f32) * s).round().max(1.0) as u32;
                    nh = ((h as f32) * s).round().max(1.0) as u32;
                }
                ResizeSpec::Width(tw) => {
                    let s = tw as f32 / w.max(1) as f32;
                    nw = tw.max(1);
                    nh = ((h as f32) * s).round().max(1.0) as u32;
                }
                ResizeSpec::Height(th) => {
                    let s = th as f32 / h.max(1) as f32;
                    nh = th.max(1);
                    nw = ((w as f32) * s).round().max(1.0) as u32;
                }
                ResizeSpec::Fit(mw, mh) => {
                    let s = (mw as f32 / w.max(1) as f32).min(mh as f32 / h.max(1) as f32);
                    nw = ((w as f32) * s).round().max(1.0) as u32;
                    nh = ((h as f32) * s).round().max(1.0) as u32;
                }
            }
        }
        if let Some(max) = self.max {
            let long = nw.max(nh);
            if long > max {
                let s = max as f32 / long as f32;
                nw = ((nw as f32) * s).round().max(1.0) as u32;
                nh = ((nh as f32) * s).round().max(1.0) as u32;
            }
        }
        (nw.max(1), nh.max(1))
    }
}

pub fn parse_resize(s: &str) -> Result<ResizeSpec, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty --resize".into());
    }
    if let Some(rest) = s.strip_suffix('%') {
        let p: f32 = rest
            .trim()
            .parse()
            .map_err(|_| format!("invalid --resize {s}"))?;
        if p <= 0.0 {
            return Err("--resize percent must be > 0".into());
        }
        return Ok(ResizeSpec::Percent(p));
    }
    let lower = s.to_ascii_lowercase();
    if let Some((w, h)) = lower.split_once('x') {
        let w = w.trim();
        let h = h.trim();
        return match (w.is_empty(), h.is_empty()) {
            (false, false) => {
                let ww = parse_px(w, s)?;
                let hh = parse_px(h, s)?;
                Ok(ResizeSpec::Fit(ww, hh))
            }
            (false, true) => Ok(ResizeSpec::Width(parse_px(w, s)?)),
            (true, false) => Ok(ResizeSpec::Height(parse_px(h, s)?)),
            (true, true) => Err("invalid --resize".into()),
        };
    }
    Ok(ResizeSpec::Width(parse_px(s, s)?))
}

fn parse_px(n: &str, orig: &str) -> Result<u32, String> {
    let v: u32 = n
        .parse()
        .map_err(|_| format!("invalid --resize {orig} (use 50%, 800x, x600, 800x600)"))?;
    if v == 0 {
        return Err("--resize dimensions must be > 0".into());
    }
    Ok(v)
}

/// ffmpeg `-vf` chain for `--resize` / `--max`. None if nothing to do.
pub fn ffmpeg_scale_filter(g: &Geometry) -> Option<String> {
    if g.is_noop() {
        return None;
    }
    let mut parts = Vec::new();
    if let Some(r) = &g.resize {
        parts.push(match r {
            ResizeSpec::Percent(p) => {
                let f = p / 100.0;
                format!("scale=iw*{f}:ih*{f}")
            }
            ResizeSpec::Width(w) => format!("scale={w}:-2"),
            ResizeSpec::Height(h) => format!("scale=-2:{h}"),
            ResizeSpec::Fit(w, h) => {
                format!("scale={w}:{h}:force_original_aspect_ratio=decrease")
            }
        });
    }
    if let Some(max) = g.max {
        parts.push(format!(
            "scale=w='min(iw,{max})':h='min(ih,{max})':force_original_aspect_ratio=decrease"
        ));
    }
    Some(parts.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_specs() {
        assert_eq!(parse_resize("50%").unwrap(), ResizeSpec::Percent(50.0));
        assert_eq!(parse_resize("800x").unwrap(), ResizeSpec::Width(800));
        assert_eq!(parse_resize("x600").unwrap(), ResizeSpec::Height(600));
        assert_eq!(parse_resize("800x600").unwrap(), ResizeSpec::Fit(800, 600));
        assert_eq!(parse_resize("1920").unwrap(), ResizeSpec::Width(1920));
        assert!(parse_resize("0%").is_err());
        assert!(parse_resize("foo").is_err());
    }

    #[test]
    fn max_no_upscale() {
        let g = Geometry {
            max: Some(1920),
            resize: None,
        };
        assert_eq!(g.target_size(4000, 3000), (1920, 1440));
        assert_eq!(g.target_size(800, 600), (800, 600));
        assert_eq!(g.target_size(1000, 4000), (480, 1920));
    }

    #[test]
    fn percent_and_fit() {
        let g = Geometry {
            max: None,
            resize: Some(ResizeSpec::Percent(50.0)),
        };
        assert_eq!(g.target_size(100, 80), (50, 40));
        let g = Geometry {
            max: Some(100),
            resize: Some(ResizeSpec::Percent(50.0)),
        };
        assert_eq!(g.target_size(400, 400), (100, 100));
    }
}
