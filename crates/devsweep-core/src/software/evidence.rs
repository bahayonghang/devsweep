//! Pure optional display evidence parsing. No filesystem reads or authority.

#[cfg(any(windows, test))]
use super::IconEvidenceV1;

#[cfg(any(windows, test))]
const MAX_ICON_PATH_BYTES: usize = 32 * 1024;

/// Validates dates from collectors, saved inventories, and injected fixtures.
/// This projection never rewrites the fingerprint-bearing inventory payload.
pub fn validated_installed_on(value: Option<&str>) -> Option<&str> {
    let value = value?;
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year = digits(&bytes[..4])?;
    let month = digits(&bytes[5..7])?;
    let day = digits(&bytes[8..])?;
    valid_date(year, month, day).then_some(value)
}

#[cfg(any(windows, test))]
pub(super) fn parse_install_date(value: Option<&str>) -> Option<String> {
    let value = value?.trim().as_bytes();
    if value.len() != 8 {
        return None;
    }
    let year = digits(&value[..4])?;
    let month = digits(&value[4..6])?;
    let day = digits(&value[6..])?;
    valid_date(year, month, day).then(|| format!("{year:04}-{month:02}-{day:02}"))
}

fn digits(bytes: &[u8]) -> Option<u32> {
    bytes.iter().try_fold(0_u32, |number, byte| {
        byte.is_ascii_digit()
            .then(|| number * 10 + u32::from(byte - b'0'))
    })
}

fn valid_date(year: u32, month: u32, day: u32) -> bool {
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        _ => return false,
    };
    (1..=9999).contains(&year) && (1..=days).contains(&day)
}

#[cfg(any(windows, test))]
pub(super) fn parse_display_icon(value: Option<&str>) -> Option<IconEvidenceV1> {
    let value = value?.trim();
    if value.is_empty() || value.len() > MAX_ICON_PATH_BYTES || value.chars().any(char::is_control)
    {
        return None;
    }
    let (path, index) = if let Some(quoted) = value.strip_prefix('"') {
        let (path, tail) = quoted.split_once('"')?;
        let tail = tail.trim();
        let index = if tail.is_empty() {
            0
        } else {
            parse_index(tail.strip_prefix(',')?.trim())?
        };
        (path, index)
    } else if let Some((path, index)) = value.rsplit_once(',') {
        if path.contains(',') {
            return None;
        }
        (path.trim_end(), parse_index(index.trim())?)
    } else {
        (value, 0)
    };
    // An unquoted resource name followed by whitespace may be command text.
    // Quoting removes that ambiguity without probing the filesystem.
    if path
        .rsplit(['\\', '/'])
        .next()?
        .rsplit_once('.')
        .is_some_and(|(_, extension)| extension.chars().any(char::is_whitespace))
        || (!value.starts_with('"')
            && path.char_indices().any(|(index, ch)| {
                ch.is_whitespace()
                    && path[..index]
                        .rsplit_once('.')
                        .is_some_and(|(_, extension)| {
                            ["exe", "dll", "ico", "png"]
                                .iter()
                                .any(|known| extension.eq_ignore_ascii_case(known))
                        })
            }))
    {
        return None;
    }
    local_icon_path(path).map(|path| IconEvidenceV1 { path, index })
}

#[cfg(any(windows, test))]
fn parse_index(value: &str) -> Option<i32> {
    let digits = value.strip_prefix(['+', '-']).unwrap_or(value);
    (!digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| value.parse().ok())
        .flatten()
}

#[cfg(any(windows, test))]
fn local_icon_path(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    if bytes.len() < 4
        || value.len() > MAX_ICON_PATH_BYTES
        || !bytes[0].is_ascii_alphabetic()
        || bytes[1] != b':'
        || !matches!(bytes[2], b'\\' | b'/')
        || value
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '"' | '%' | '<' | '>' | '|' | '?' | '*'))
        || value[2..].contains(':')
    {
        return None;
    }
    let path = value.replace('/', "\\");
    if path[3..]
        .split('\\')
        .any(|part| part.is_empty() || matches!(part, "." | "..") || part.ends_with(['.', ' ']))
    {
        return None;
    }
    Some(path)
}

/// A resolved MSIX logo URI must identify a local file, never a network asset.
#[cfg(any(windows, test))]
pub(super) fn parse_msix_logo(value: Option<&str>) -> Option<IconEvidenceV1> {
    let uri = value?;
    if uri.len() > MAX_ICON_PATH_BYTES || uri.contains(['?', '#']) {
        return None;
    }
    let encoded = uri.strip_prefix("file:///")?;
    let mut decoded = Vec::with_capacity(encoded.len());
    let mut bytes = encoded.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let high = char::from(bytes.next()?).to_digit(16)?;
            let low = char::from(bytes.next()?).to_digit(16)?;
            decoded.push(u8::try_from(high * 16 + low).ok()?);
        } else {
            decoded.push(byte);
        }
    }
    let path = String::from_utf8(decoded).ok()?;
    local_icon_path(&path).map(|path| IconEvidenceV1 { path, index: 0 })
}

/// WinRT ticks are 100 ns since 1601-01-01; project the UTC civil day only.
#[cfg(any(windows, test))]
pub(super) fn parse_msix_installed_date(ticks: Option<i64>) -> Option<String> {
    let ticks = ticks?;
    if ticks == 0 {
        return None;
    }
    let unix_days = ticks.div_euclid(864_000_000_000).checked_sub(134_774)?;
    // Gregorian civil-from-days, with March as the first month of each era.
    let shifted = unix_days.checked_add(719_468)?;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let march_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * march_month + 2) / 5 + 1;
    let month = march_month + if march_month < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    if !(1..=9999).contains(&year) {
        return None;
    }
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_formats_preserve_resource_indices_without_reading_paths() {
        for (raw, path, index) in [
            (r"C:\Apps\app.exe,0", r"C:\Apps\app.exe", 0),
            (r"C:\missing\app.exe", r"C:\missing\app.exe", 0),
            (
                r#""C:\Apps\Acme, Inc\app.exe",-101"#,
                r"C:\Apps\Acme, Inc\app.exe",
                -101,
            ),
            (r#" "C:\Apps\app.ico" "#, r"C:\Apps\app.ico", 0),
            (r"C:\Apps\app.dll,+2", r"C:\Apps\app.dll", 2),
            (
                r"C:\Program Files\App icon.ico",
                r"C:\Program Files\App icon.ico",
                0,
            ),
            (r"C:\Apps\app.v2 icon.ico", r"C:\Apps\app.v2 icon.ico", 0),
            (
                r#""C:\Apps\app.exe icon, final.ico",0"#,
                r"C:\Apps\app.exe icon, final.ico",
                0,
            ),
        ] {
            assert_eq!(
                parse_display_icon(Some(raw)),
                Some(IconEvidenceV1 {
                    path: path.into(),
                    index
                })
            );
        }
        for raw in [
            "",
            r#""C:\Apps\app.exe" --flag"#,
            r"C:\Apps\app.exe --flag",
            r"C:\Apps\app.exe --flag=x.txt",
            r"C:\Apps\app.EXE /quiet",
            r"C:\Apps\app.dll /resource=x.ico,0",
            r#""C:\Apps\app.exe"#,
            r"C:\Apps\app.exe,oops",
            r"C:\Apps\app.exe,2147483648",
            r"C:\Apps\Acme, Inc\app.exe,0",
            r"%LOCALAPPDATA%\app.exe,0",
            r"\\server\app.ico",
            r"\\?\C:\app.ico",
            r"C:app.ico",
            r"app.ico",
            r"https://example.com/app.ico",
            r"C:\app.ico:stream",
            r"C:\..\app.ico",
            "C:\\app\0.ico",
        ] {
            assert_eq!(parse_display_icon(Some(raw)), None, "{raw:?}");
        }
        assert_eq!(parse_display_icon(None), None);
    }

    #[test]
    fn installer_dates_and_saved_date_projection_are_strict() {
        for (raw, expected) in [
            ("20261104", "2026-11-04"),
            (" 20240229 ", "2024-02-29"),
            ("00010101", "0001-01-01"),
        ] {
            assert_eq!(parse_install_date(Some(raw)).as_deref(), Some(expected));
            assert_eq!(validated_installed_on(Some(expected)), Some(expected));
        }
        for raw in [
            "",
            "20260230",
            "20230229",
            "19000229",
            "00000101",
            "2026/11/04",
            "Tue Jun 17 16:17:16 2025",
            "２０２６１１０４",
        ] {
            assert_eq!(parse_install_date(Some(raw)), None, "{raw}");
        }
        for raw in [
            "2026-02-30",
            "2026-1-01",
            "2026-01-01 ",
            "bad",
            "0000-01-01",
            "2026-01-01T00:00:00Z",
        ] {
            assert_eq!(validated_installed_on(Some(raw)), None, "{raw}");
        }
        assert_eq!(parse_install_date(None), None);
        assert_eq!(validated_installed_on(None), None);
    }

    #[test]
    fn msix_logo_is_decoded_without_resolving_network_or_manifest_paths() {
        assert_eq!(
            parse_msix_logo(Some(
                "file:///C:/Program%20Files/WindowsApps/split/Logo.png"
            )),
            Some(IconEvidenceV1 {
                path: r"C:\Program Files\WindowsApps\split\Logo.png".into(),
                index: 0
            })
        );
        for raw in [
            "file://server/share/icon.png",
            "ms-appx:///Assets/Logo.png",
            "ms-resource:Logo",
            "https://example.com/a.png",
            "file:///C:/a.png?query",
            "file:///C:/a.png#fragment",
            "file:///C:/%00.png",
            "file:///C:/%zz.png",
            "file:///C:/%FF.png",
            "file:///C:/../a.png",
        ] {
            assert_eq!(parse_msix_logo(Some(raw)), None, "{raw}");
        }
        assert_eq!(parse_msix_logo(None), None);
    }

    #[test]
    fn msix_dates_use_utc_days_not_a_local_offset() {
        let midnight = 134_075_520_000_000_000; // 2025-11-14T00:00:00Z
        assert_eq!(
            parse_msix_installed_date(Some(midnight)).as_deref(),
            Some("2025-11-14")
        );
        assert_eq!(
            parse_msix_installed_date(Some(midnight - 1)).as_deref(),
            Some("2025-11-13")
        );
        assert_eq!(
            parse_msix_installed_date(Some(116_444_736_000_000_000)).as_deref(),
            Some("1970-01-01")
        );
        assert_eq!(parse_msix_installed_date(Some(0)), None);
        assert_eq!(parse_msix_installed_date(Some(i64::MAX)), None);
        assert_eq!(parse_msix_installed_date(None), None);
    }
}
