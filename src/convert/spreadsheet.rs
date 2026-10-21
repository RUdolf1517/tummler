use crate::error::TummlerError;
use crate::util::normalize_ext;
use calamine::{open_workbook_auto, Data, Reader};
use rust_xlsxwriter::{Workbook, Worksheet, XlsxError};
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

/// csv / txt — the spreadsheet export targets.
pub fn is_tabular_ext(ext: &str) -> bool {
    matches!(normalize_ext(ext).as_str(), "csv" | "txt")
}

pub fn is_xlsx_ext(ext: &str) -> bool {
    normalize_ext(ext) == "xlsx"
}

/// csv / xls / ods / … → xlsx (not xlsx → xlsx).
pub fn can_write_xlsx(src: &str) -> bool {
    let src = normalize_ext(src);
    src == "csv"
        || matches!(src.as_str(), "xls" | "xlsm" | "xlsb" | "ods")
}

fn map_calamine(e: calamine::Error) -> TummlerError {
    let msg = e.to_string();
    if msg.to_lowercase().contains("password") {
        TummlerError::SpreadsheetError("encrypted workbook (password protected)".into())
    } else {
        TummlerError::SpreadsheetError(msg)
    }
}

fn list_sheets(path: &Path) -> Result<Vec<String>, TummlerError> {
    let workbook = open_workbook_auto(path).map_err(map_calamine)?;
    let names = workbook.sheet_names();
    if names.is_empty() {
        return Err(TummlerError::SpreadsheetError("empty workbook".into()));
    }
    Ok(names)
}

/// Pick output paths. `--all-sheets` → one file per sheet (`stem-SheetName.ext`).
pub fn plan_outputs(
    input: &Path,
    base: &Path,
    all_sheets: bool,
    sheet: Option<&str>,
) -> Result<Vec<(PathBuf, Option<String>)>, TummlerError> {
    if !all_sheets {
        return Ok(vec![(base.to_path_buf(), sheet.map(str::to_string))]);
    }
    let names = list_sheets(input)?;
    let mut used = HashSet::new();
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let safe = unique_filename(&mut used, &sanitize_sheet_filename(&name));
        out.push((with_sheet_suffix(base, &safe), Some(name)));
    }
    Ok(out)
}

pub fn sanitize_sheet_filename(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    s = s.trim().trim_matches('.').to_string();
    if s.is_empty() {
        s = "sheet".into();
    }
    if s.len() > 80 {
        s.truncate(80);
    }
    s
}

fn unique_filename(used: &mut HashSet<String>, base: &str) -> String {
    let mut name = base.to_string();
    let mut i = 2;
    while used.contains(&name) {
        name = format!("{base}-{i}");
        i += 1;
    }
    used.insert(name.clone());
    name
}

fn with_sheet_suffix(path: &Path, sheet_file: &str) -> PathBuf {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("csv");
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    parent.join(format!("{stem}-{sheet_file}.{ext}"))
}

fn resolve_sheet_name(names: &[String], want: Option<&str>) -> Result<String, TummlerError> {
    if names.is_empty() {
        return Err(TummlerError::SpreadsheetError("empty workbook".into()));
    }
    match want {
        None => Ok(names[0].clone()),
        Some(want) => names
            .iter()
            .find(|n| n.as_str() == want)
            .or_else(|| names.iter().find(|n| n.eq_ignore_ascii_case(want)))
            .cloned()
            .ok_or_else(|| {
                TummlerError::SpreadsheetError(format!(
                    "unknown sheet: {want} (available: {})",
                    names.join(", ")
                ))
            }),
    }
}

fn format_number(f: f64) -> String {
    if f.is_finite() && f.abs() < 1e15 && (f - f.round()).abs() <= 1e-9 {
        format!("{:.0}", f.round())
    } else {
        format!("{f}")
    }
}

fn cell_text(d: &Data) -> String {
    match d {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) => format_number(*f),
        Data::Bool(b) => {
            if *b {
                "TRUE".into()
            } else {
                "FALSE".into()
            }
        }
        Data::DateTime(dt) => dt.to_string(),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(e) => e.to_string(),
    }
}

fn read_sheet(path: &Path, sheet: Option<&str>) -> Result<Vec<Vec<String>>, TummlerError> {
    let mut workbook = open_workbook_auto(path).map_err(map_calamine)?;
    let names = workbook.sheet_names();
    let name = resolve_sheet_name(&names, sheet)?;
    let range = workbook.worksheet_range(&name).map_err(map_calamine)?;
    if range.is_empty() {
        return Ok(Vec::new());
    }
    let mut rows: Vec<Vec<String>> = range
        .rows()
        .map(|row| row.iter().map(cell_text).collect())
        .collect();
    trim_table(&mut rows);
    Ok(rows)
}

fn trim_table(rows: &mut Vec<Vec<String>>) {
    while rows.last().is_some_and(|r| r.iter().all(|c| c.is_empty())) {
        rows.pop();
    }
    let mut last = 0usize;
    for row in rows.iter() {
        for (i, c) in row.iter().enumerate() {
            if !c.is_empty() {
                last = last.max(i + 1);
            }
        }
    }
    for row in rows.iter_mut() {
        row.truncate(last);
    }
}

fn write_csv_cell(f: &mut impl Write, s: &str) -> std::io::Result<()> {
    let must_quote = s.contains([',', '"', '\n', '\r'])
        || s.starts_with(' ')
        || s.ends_with(' ');
    if !must_quote {
        return f.write_all(s.as_bytes());
    }
    f.write_all(b"\"")?;
    let mut rest = s;
    while let Some(i) = rest.find('"') {
        f.write_all(&rest.as_bytes()[..i])?;
        f.write_all(b"\"\"")?;
        rest = &rest[i + 1..];
    }
    f.write_all(rest.as_bytes())?;
    f.write_all(b"\"")
}

fn write_csv(rows: &[Vec<String>], f: &mut impl Write) -> std::io::Result<()> {
    for row in rows {
        for (i, c) in row.iter().enumerate() {
            if i > 0 {
                f.write_all(b",")?;
            }
            write_csv_cell(f, c)?;
        }
        f.write_all(b"\n")?;
    }
    Ok(())
}

fn write_txt(rows: &[Vec<String>], f: &mut impl Write) -> std::io::Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    let ncols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut widths = vec![0usize; ncols];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    for row in rows {
        for (i, w) in widths.iter().enumerate() {
            if i > 0 {
                f.write_all(b"  ")?;
            }
            let cell = row.get(i).map(String::as_str).unwrap_or("");
            f.write_all(cell.as_bytes())?;
            if i + 1 < ncols {
                let mut pad = w.saturating_sub(cell.chars().count());
                const SPACES: &[u8] = &[b' '; 64];
                while pad > 0 {
                    let n = pad.min(SPACES.len());
                    f.write_all(&SPACES[..n])?;
                    pad -= n;
                }
            }
        }
        f.write_all(b"\n")?;
    }
    Ok(())
}

/// Spreadsheet (xlsx/xls/ods/…) → csv or aligned txt.
pub fn convert_spreadsheet(
    input: &Path,
    output: &Path,
    target_ext: &str,
    sheet: Option<&str>,
) -> Result<(), TummlerError> {
    let ext = normalize_ext(target_ext);
    if !is_tabular_ext(&ext) {
        return Err(TummlerError::UnsupportedConversion(
            crate::util::get_extension(input).unwrap_or_default(),
            ext,
        ));
    }
    let rows = read_sheet(input, sheet)?;
    let mut f = std::io::BufWriter::with_capacity(
        64 * 1024,
        std::fs::File::create(output).map_err(|e| TummlerError::IoError(e.to_string()))?,
    );
    let write = if ext == "csv" {
        write_csv(&rows, &mut f)
    } else {
        write_txt(&rows, &mut f)
    };
    write.map_err(|e| TummlerError::IoError(e.to_string()))?;
    f.flush().map_err(|e| TummlerError::IoError(e.to_string()))?;
    Ok(())
}

fn map_xlsx(e: XlsxError) -> TummlerError {
    TummlerError::SpreadsheetError(e.to_string())
}

fn excel_sheet_name(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| match c {
            ':' | '\\' | '/' | '?' | '*' | '[' | ']' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    s = s.trim().trim_matches('\'').to_string();
    if s.is_empty() {
        s = "Sheet1".into();
    }
    const MAX: usize = 31;
    if s.chars().count() > MAX {
        s = s.chars().take(MAX).collect();
    }
    s
}

fn looks_like_number(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() {
        return false;
    }
    let rest = t.strip_prefix('-').unwrap_or(t);
    if rest.len() > 1 && rest.starts_with('0') && !rest.starts_with("0.") {
        return false;
    }
    t.parse::<f64>().is_ok()
}

fn write_csv_field(
    ws: &mut Worksheet,
    row: u32,
    col: u16,
    s: &str,
) -> Result<(), XlsxError> {
    if s.is_empty() {
        return Ok(());
    }
    if s == "TRUE" {
        ws.write_boolean(row, col, true)?;
        return Ok(());
    }
    if s == "FALSE" {
        ws.write_boolean(row, col, false)?;
        return Ok(());
    }
    if looks_like_number(s)
        && let Ok(n) = s.trim().parse::<f64>()
        && n.is_finite()
    {
        ws.write_number(row, col, n)?;
        return Ok(());
    }
    ws.write_string(row, col, s)?;
    Ok(())
}

fn write_data_cell(
    ws: &mut Worksheet,
    row: u32,
    col: u16,
    d: &Data,
) -> Result<(), XlsxError> {
    match d {
        Data::Empty => Ok(()),
        Data::String(s) => {
            ws.write_string(row, col, s)?;
            Ok(())
        }
        Data::Int(i) => {
            ws.write_number(row, col, *i as f64)?;
            Ok(())
        }
        Data::Float(f) => {
            ws.write_number(row, col, *f)?;
            Ok(())
        }
        Data::Bool(b) => {
            ws.write_boolean(row, col, *b)?;
            Ok(())
        }
        Data::DateTime(dt) => {
            ws.write_string(row, col, dt.to_string())?;
            Ok(())
        }
        Data::DateTimeIso(s) | Data::DurationIso(s) => {
            ws.write_string(row, col, s)?;
            Ok(())
        }
        Data::Error(e) => {
            ws.write_string(row, col, e.to_string())?;
            Ok(())
        }
    }
}

fn add_named_sheet<'a>(
    workbook: &'a mut Workbook,
    used: &mut HashSet<String>,
    want: &str,
) -> Result<&'a mut Worksheet, TummlerError> {
    let name = unique_filename(used, &excel_sheet_name(want));
    let ws = workbook.add_worksheet();
    ws.set_name(&name).map_err(map_xlsx)?;
    Ok(ws)
}

fn write_csv_sheet(ws: &mut Worksheet, path: &Path) -> Result<(), TummlerError> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(path)
        .map_err(|e| TummlerError::SpreadsheetError(e.to_string()))?;
    for (ri, rec) in rdr.records().enumerate() {
        let rec = rec.map_err(|e| TummlerError::SpreadsheetError(e.to_string()))?;
        let row = u32::try_from(ri).map_err(|_| {
            TummlerError::SpreadsheetError("too many rows for xlsx".into())
        })?;
        if rec.len() > 16384 {
            return Err(TummlerError::SpreadsheetError(
                "too many columns for xlsx".into(),
            ));
        }
        for (ci, field) in rec.iter().enumerate() {
            let col = ci as u16;
            write_csv_field(ws, row, col, field).map_err(map_xlsx)?;
        }
    }
    Ok(())
}

fn write_calamine_sheet(
    ws: &mut Worksheet,
    range: &calamine::Range<Data>,
) -> Result<(), TummlerError> {
    if range.is_empty() {
        return Ok(());
    }
    for (ri, row) in range.rows().enumerate() {
        let row_i = u32::try_from(ri).map_err(|_| {
            TummlerError::SpreadsheetError("too many rows for xlsx".into())
        })?;
        if row.len() > 16384 {
            return Err(TummlerError::SpreadsheetError(
                "too many columns for xlsx".into(),
            ));
        }
        for (ci, cell) in row.iter().enumerate() {
            write_data_cell(ws, row_i, ci as u16, cell).map_err(map_xlsx)?;
        }
    }
    Ok(())
}

/// csv / xls / ods / … → xlsx. No LibreOffice.
pub fn convert_to_xlsx(
    input: &Path,
    output: &Path,
    sheet: Option<&str>,
) -> Result<(), TummlerError> {
    let src = normalize_ext(&crate::util::get_extension(input).unwrap_or_default());
    if !can_write_xlsx(&src) {
        return Err(TummlerError::UnsupportedConversion(
            src,
            "xlsx".into(),
        ));
    }

    let mut workbook = Workbook::new();
    let mut used = HashSet::new();

    if src == "csv" {
        let name = sheet.unwrap_or("Sheet1");
        let ws = add_named_sheet(&mut workbook, &mut used, name)?;
        write_csv_sheet(ws, input)?;
    } else {
        let mut src_wb = open_workbook_auto(input).map_err(map_calamine)?;
        let names = src_wb.sheet_names();
        if names.is_empty() {
            return Err(TummlerError::SpreadsheetError("empty workbook".into()));
        }
        let names = if let Some(want) = sheet {
            vec![resolve_sheet_name(&names, Some(want))?]
        } else {
            names
        };
        for name in names {
            let range = src_wb.worksheet_range(&name).map_err(map_calamine)?;
            let ws = add_named_sheet(&mut workbook, &mut used, &name)?;
            write_calamine_sheet(ws, &range)?;
        }
    }

    workbook.save(output).map_err(map_xlsx)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::write::SimpleFileOptions;

    fn col_name(n: usize) -> String {
        let mut n = n;
        let mut s = String::new();
        loop {
            s.insert(0, (b'A' + (n % 26) as u8) as char);
            if n < 26 {
                break;
            }
            n = n / 26 - 1;
        }
        s
    }

    fn xml_esc(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }

    /// Tiny xlsx: `sheets[i] = (name, rows of cells)`.
    /// A cell starting with `=` is a formula whose cached value follows `=>` (`=1+1=>2`).
    /// A cell starting with `#n:` is a number (`#n:42`). `#b:1` / `#b:0` is a bool.
    fn write_xlsx(path: &Path, sheets: &[(&str, &[Vec<&str>])]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let mut overrides = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
"#,
        );
        for i in 0..sheets.len() {
            overrides.push_str(&format!(
                "<Override PartName=\"/xl/worksheets/sheet{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>\n",
                i + 1
            ));
        }
        overrides.push_str("</Types>");
        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(overrides.as_bytes()).unwrap();

        zip.start_file("_rels/.rels", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#,
        )
        .unwrap();

        let mut wb = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets>
"#,
        );
        let mut rels = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
"#,
        );
        for (i, (name, _)) in sheets.iter().enumerate() {
            let id = i + 1;
            wb.push_str(&format!(
                "<sheet name=\"{}\" sheetId=\"{id}\" r:id=\"rId{id}\"/>\n",
                xml_esc(name)
            ));
            rels.push_str(&format!(
                "<Relationship Id=\"rId{id}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{id}.xml\"/>\n"
            ));
        }
        wb.push_str("</sheets></workbook>");
        rels.push_str("</Relationships>");
        zip.start_file("xl/workbook.xml", opts).unwrap();
        zip.write_all(wb.as_bytes()).unwrap();
        zip.start_file("xl/_rels/workbook.xml.rels", opts).unwrap();
        zip.write_all(rels.as_bytes()).unwrap();

        for (i, (_name, rows)) in sheets.iter().enumerate() {
            let mut sheet = String::from(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<sheetData>
"#,
            );
            for (ri, row) in rows.iter().enumerate() {
                let r = ri + 1;
                sheet.push_str(&format!("<row r=\"{r}\">"));
                for (ci, cell) in row.iter().enumerate() {
                    let addr = format!("{}{r}", col_name(ci));
                    if cell.is_empty() {
                        continue;
                    }
                    if let Some(rest) = cell.strip_prefix("#n:") {
                        sheet.push_str(&format!("<c r=\"{addr}\"><v>{}</v></c>", xml_esc(rest)));
                    } else if let Some(rest) = cell.strip_prefix("#b:") {
                        sheet.push_str(&format!(
                            "<c r=\"{addr}\" t=\"b\"><v>{}</v></c>",
                            xml_esc(rest)
                        ));
                    } else if let Some(rest) = cell.strip_prefix('=') {
                        let (formula, value) = rest.split_once("=>").unwrap_or((rest, ""));
                        sheet.push_str(&format!(
                            "<c r=\"{addr}\"><f>{}</f><v>{}</v></c>",
                            xml_esc(formula),
                            xml_esc(value)
                        ));
                    } else {
                        sheet.push_str(&format!(
                            "<c r=\"{addr}\" t=\"inlineStr\"><is><t>{}</t></is></c>",
                            xml_esc(cell)
                        ));
                    }
                }
                sheet.push_str("</row>\n");
            }
            sheet.push_str("</sheetData></worksheet>");
            zip.start_file(format!("xl/worksheets/sheet{}.xml", i + 1), opts)
                .unwrap();
            zip.write_all(sheet.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn xlsx_to_csv_first_sheet() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("table.xlsx");
        let sheet1 = vec![
            vec!["name", "qty"],
            vec!["apples", "#n:3"],
            vec!["pears", "#n:2"],
        ];
        let sheet2 = vec![vec!["secret"]];
        write_xlsx(&input, &[("Sheet1", &sheet1), ("Sales", &sheet2)]);
        let out = dir.path().join("table.csv");
        convert_spreadsheet(&input, &out, "csv", None).unwrap();
        let s = std::fs::read_to_string(&out).unwrap();
        assert_eq!(s, "name,qty\napples,3\npears,2\n");
    }

    #[test]
    fn xlsx_named_sheet_and_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("table.xlsx");
        let a = vec![vec!["A"]];
        let b = vec![vec!["B"]];
        write_xlsx(&input, &[("Sheet1", &a), ("Sales", &b)]);
        let out = dir.path().join("s.csv");
        convert_spreadsheet(&input, &out, "csv", Some("sales")).unwrap();
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "B\n");
        let err = convert_spreadsheet(&input, &out, "csv", Some("Nope")).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unknown sheet"), "{msg}");
        assert!(msg.contains("Sales"), "{msg}");
    }

    #[test]
    fn csv_quotes_comma_and_quote() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("q.xlsx");
        let rows = vec![vec!["hello, world", r#"say "hi""#]];
        write_xlsx(&input, &[("Sheet1", &rows)]);
        let out = dir.path().join("q.csv");
        convert_spreadsheet(&input, &out, "csv", None).unwrap();
        assert_eq!(
            std::fs::read_to_string(&out).unwrap(),
            "\"hello, world\",\"say \"\"hi\"\"\"\n"
        );
    }

    #[test]
    fn txt_is_aligned() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("t.xlsx");
        let rows = vec![vec!["a", "bb"], vec!["ccc", "d"]];
        write_xlsx(&input, &[("Sheet1", &rows)]);
        let out = dir.path().join("t.txt");
        convert_spreadsheet(&input, &out, "txt", None).unwrap();
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "a    bb\nccc  d\n");
    }

    #[test]
    fn formula_uses_cached_value() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("f.xlsx");
        let rows = vec![vec!["=1+1=>2"]];
        write_xlsx(&input, &[("Sheet1", &rows)]);
        let out = dir.path().join("f.csv");
        convert_spreadsheet(&input, &out, "csv", None).unwrap();
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "2\n");
    }

    #[test]
    fn bool_and_empty_cells() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("b.xlsx");
        let rows = vec![vec!["#b:1", "", "x"]];
        write_xlsx(&input, &[("Sheet1", &rows)]);
        let out = dir.path().join("b.csv");
        convert_spreadsheet(&input, &out, "csv", None).unwrap();
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "TRUE,,x\n");
    }

    #[test]
    fn all_sheets_plan_suffixes() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("table.xlsx");
        let a = vec![vec!["A"]];
        let b = vec![vec!["B"]];
        write_xlsx(&input, &[("Sheet1", &a), ("Sales/Q1", &b)]);
        let base = dir.path().join("table.csv");
        let plan = plan_outputs(&input, &base, true, None).unwrap();
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].0.file_name().unwrap(), "table-Sheet1.csv");
        assert_eq!(plan[1].0.file_name().unwrap(), "table-Sales_Q1.csv");
        assert_eq!(plan[1].1.as_deref(), Some("Sales/Q1"));
    }

    #[test]
    fn empty_sheet_writes_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("e.xlsx");
        let empty: Vec<Vec<&str>> = vec![];
        write_xlsx(&input, &[("Sheet1", &empty)]);
        let out = dir.path().join("e.csv");
        convert_spreadsheet(&input, &out, "csv", None).unwrap();
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "");
    }

    #[test]
    fn sanitize_unsafe_sheet_names() {
        assert_eq!(sanitize_sheet_filename("Sales/Q1"), "Sales_Q1");
        assert_eq!(sanitize_sheet_filename("  ..  "), "sheet");
        assert_eq!(sanitize_sheet_filename("ok"), "ok");
    }

    #[test]
    fn empty_workbook_errors() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("empty.xlsx");
        let none: &[(&str, &[Vec<&str>])] = &[];
        write_xlsx(&input, none);
        let out = dir.path().join("e.csv");
        assert!(convert_spreadsheet(&input, &out, "csv", None).is_err());
    }

    #[test]
    fn rejects_non_tabular_target() {
        let err = convert_spreadsheet(Path::new("x.xlsx"), Path::new("x.png"), "png", None)
            .unwrap_err();
        match err {
            TummlerError::UnsupportedConversion(_, t) => assert_eq!(t, "png"),
            other => panic!("{other}"),
        }
    }

    #[test]
    fn csv_to_xlsx_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let csv = dir.path().join("t.csv");
        std::fs::write(&csv, "name,qty\napples,3\n\"hello, world\",TRUE\n").unwrap();
        let out = dir.path().join("t.xlsx");
        convert_to_xlsx(&csv, &out, None).unwrap();
        let rows = read_sheet(&out, None).unwrap();
        assert_eq!(rows[0], vec!["name", "qty"]);
        assert_eq!(rows[1], vec!["apples", "3"]);
        assert_eq!(rows[2], vec!["hello, world", "TRUE"]);
    }

    #[test]
    fn csv_to_xlsx_named_sheet() {
        let dir = tempfile::tempdir().unwrap();
        let csv = dir.path().join("t.csv");
        std::fs::write(&csv, "a\n").unwrap();
        let out = dir.path().join("t.xlsx");
        convert_to_xlsx(&csv, &out, Some("Sales/Q1")).unwrap();
        let wb = open_workbook_auto(&out).unwrap();
        assert_eq!(wb.sheet_names(), vec!["Sales_Q1"]);
    }

    #[test]
    fn xlsx_write_sources() {
        assert!(can_write_xlsx("csv"));
        assert!(can_write_xlsx("xls"));
        assert!(can_write_xlsx("ods"));
        assert!(can_write_xlsx("xlsm"));
        assert!(!can_write_xlsx("xlsx"));
        assert!(!can_write_xlsx("txt"));
        assert!(!can_write_xlsx("png"));
        assert!(is_xlsx_ext("XLSX"));
    }
}
