use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::managers::winget::{
    non_empty, parse_powershell_rows, parse_rows, powershell_fallback_warning, read_table,
    run_powershell, strip_terminal_sequences, table_fields,
};
use crate::models::{ManagerError, ManagerId};
use crate::process::{self, CommandOutput, LIST_TIMEOUT, UPDATE_TIMEOUT};

const NO_PACKAGE_FOUND: u32 = 0x8A150014;
const MAX_QUERY_LENGTH: usize = 100;
const MAX_RESULTS: &str = "50";
const WINGET_SOURCE: &str = "winget";

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPackage {
    pub id: String,
    pub name: String,
    pub version: String,
    pub source: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSearchResult {
    pub packages: Vec<CatalogPackage>,
    pub errors: Vec<ManagerError>,
}

impl CatalogSearchResult {
    fn new(packages: Vec<CatalogPackage>, warnings: Vec<String>) -> Self {
        let errors = warnings
            .into_iter()
            .map(|message| ManagerError {
                manager: ManagerId::Winget,
                message,
            })
            .collect();
        Self { packages, errors }
    }
}

pub fn search(query: &str) -> AppResult<CatalogSearchResult> {
    let query = checked_query(query)?;
    let mut warnings = Vec::new();
    if let Ok(output) = run_powershell(&powershell_search(query))
        && output.success
    {
        match parse_powershell_search(&output.stdout) {
            Ok(packages) => return Ok(CatalogSearchResult::new(packages, warnings)),
            Err(error) => warnings.push(powershell_fallback_warning(&error)),
        }
    }
    let output = process::run("winget", &search_args(query), LIST_TIMEOUT)?;
    if no_package_found(&output) {
        return Ok(CatalogSearchResult::new(Vec::new(), warnings));
    }
    let output = output.ensure_success("winget search")?;
    let (packages, table_warnings) = parse_winget_search(&output.stdout)?;
    warnings.extend(table_warnings);
    Ok(CatalogSearchResult::new(packages, warnings))
}

pub fn install(package_id: &str) -> AppResult<()> {
    process::validate_package_id(package_id)?;
    process::run("winget", &install_args(package_id), UPDATE_TIMEOUT)?
        .ensure_success("winget install")?;
    Ok(())
}

/// Returns the trimmed query. A leading dash would be read as a `winget` option.
fn checked_query(query: &str) -> AppResult<&str> {
    let query = query.trim();
    let valid = !query.is_empty()
        && query.chars().count() <= MAX_QUERY_LENGTH
        && !query.starts_with('-')
        && !query.chars().any(char::is_control);
    if valid {
        return Ok(query);
    }
    Err(AppError::Other(format!("Invalid search query: {query:?}")))
}

fn search_args(query: &str) -> Vec<&str> {
    vec![
        "search",
        "--query",
        query,
        "--source",
        WINGET_SOURCE,
        "--count",
        MAX_RESULTS,
        "--accept-source-agreements",
        "--disable-interactivity",
    ]
}

fn install_args(id: &str) -> Vec<&str> {
    vec![
        "install",
        "--id",
        id,
        "--exact",
        "--source",
        WINGET_SOURCE,
        "--silent",
        "--accept-package-agreements",
        "--accept-source-agreements",
        "--disable-interactivity",
    ]
}

fn powershell_search(query: &str) -> String {
    format!(
        "$packages=@(Find-WinGetPackage -Query ({}) -Source {WINGET_SOURCE} -Count {MAX_RESULTS} | Select-Object Id,Name,Version,Source); ConvertTo-Json -InputObject $packages -Compress",
        powershell_text(query)
    )
}

/// The text is spelled as UTF-16 code units, so no character can end the literal and inject code.
fn powershell_text(text: &str) -> String {
    let units: Vec<_> = text.encode_utf16().map(|unit| unit.to_string()).collect();
    format!("[string]::new([char[]]({}))", units.join(","))
}

fn no_package_found(output: &CommandOutput) -> bool {
    output.exit_code.map(|code| code as u32) == Some(NO_PACKAGE_FOUND)
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PowershellSearchRow {
    id: String,
    name: String,
    version: Option<String>,
    source: Option<String>,
}

fn parse_powershell_search(output: &str) -> AppResult<Vec<CatalogPackage>> {
    Ok(parse_powershell_rows::<PowershellSearchRow>(output)?
        .into_iter()
        .map(|row| CatalogPackage {
            id: row.id,
            name: row.name,
            version: row.version.unwrap_or_else(|| "Unknown".into()),
            source: non_empty(row.source).unwrap_or_else(|| WINGET_SOURCE.into()),
        })
        .collect())
}

/// Besides name, ID and version winget may print a match column (e.g. "Tag: git") and a source.
fn parse_winget_search(output: &str) -> AppResult<(Vec<CatalogPackage>, Vec<String>)> {
    let clean = strip_terminal_sequences(output);
    let Some(table) = read_table(&clean)? else {
        return Ok((Vec::new(), Vec::new()));
    };
    if !(3..=5).contains(&table.columns.len()) {
        return Err(AppError::Parse(
            "WinGet: three to five table columns expected".into(),
        ));
    }
    Ok(parse_rows(&table, parse_search_row, |package| &package.id))
}

/// Rejects the notice winget prints below a truncated list, which would otherwise look like a row.
fn parse_search_row(line: &str, columns: &[usize]) -> Option<CatalogPackage> {
    let fields = table_fields(line, columns);
    let single_word = |field: &str| field.split_whitespace().count() == 1;
    if fields[0].is_empty() || !single_word(&fields[1]) || !single_word(&fields[2]) {
        return None;
    }
    Some(CatalogPackage {
        id: fields[1].clone(),
        name: fields[0].clone(),
        version: fields[2].clone(),
        source: WINGET_SOURCE.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GERMAN_TABLE: &str = include_str!("../tests/fixtures/winget_search.txt");

    #[test]
    fn parses_real_german_table_and_ignores_the_truncation_notice() {
        let (packages, warnings) = parse_winget_search(GERMAN_TABLE).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(packages.len(), 12);
        assert_eq!(packages[0].id, "Git.Git");
        assert_eq!(packages[0].name, "Git");
        assert_eq!(packages[0].version, "2.56.0.3");
        assert_eq!(packages[0].source, "winget");
        assert_eq!(packages[1].version, "2.56.0.0.2");
        assert_eq!(packages[2].id, "4nrry.OpenCherryDesktop");
        assert_eq!(packages[2].name, "OpenCherry Desktop");
        assert_eq!(packages[11].id, "Atlassian.Sourcetree");
    }

    #[test]
    fn parses_english_table_without_match_column() {
        let table = "Name    ID            Version\n-----------------------------\nTodoist Doist.Todoist 9.35.0\n";
        let (packages, _) = parse_winget_search(table).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].id, "Doist.Todoist");
        assert_eq!(packages[0].version, "9.35.0");
    }

    #[test]
    fn parses_table_with_source_column() {
        let table = "Name  Id           Version Match    Source\n------------------------------------------\nVLC   VideoLAN.VLC 3.0.24  Tag: vlc winget\n";
        let (packages, _) = parse_winget_search(table).unwrap();
        assert_eq!(packages[0].id, "VideoLAN.VLC");
        assert_eq!(packages[0].version, "3.0.24");
    }

    #[test]
    fn skips_truncated_ids_with_warning() {
        let table = "Name   ID                Version\n--------------------------------\nVendor Vendor.Complete   1.0\nBroken Vendor.Trunc…     2.0\n";
        let (packages, warnings) = parse_winget_search(table).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].id, "Vendor.Complete");
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn output_without_table_is_an_error() {
        assert!(parse_winget_search("The source could not be opened").is_err());
        assert!(parse_winget_search("Name Id\n-------\nGit Git.Git").is_err());
    }

    #[test]
    fn no_match_exit_code_is_recognized_in_signed_form() {
        let mut output = CommandOutput {
            success: false,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(-1978335212),
        };
        assert!(no_package_found(&output));
        output.exit_code = Some(1);
        assert!(!no_package_found(&output));
    }

    #[test]
    fn parses_powershell_search_rows() {
        let output = r#"[{"Id":"VideoLAN.VLC","Name":"VLC media player","Version":"3.0.24","Source":"winget"},{"Id":"Vendor.Bare","Name":"Bare","Version":null,"Source":""}]"#;
        let packages = parse_powershell_search(output).unwrap();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "VLC media player");
        assert_eq!(packages[1].version, "Unknown");
        assert_eq!(packages[1].source, "winget");
        assert!(parse_powershell_search("[]").unwrap().is_empty());
        assert!(parse_powershell_search("not json").is_err());
    }

    #[test]
    fn query_is_trimmed_and_limited() {
        assert_eq!(checked_query("  visual studio ").unwrap(), "visual studio");
        assert_eq!(checked_query("notepad++").unwrap(), "notepad++");
        assert!(checked_query(&"a".repeat(MAX_QUERY_LENGTH)).is_ok());
        assert!(checked_query(&"a".repeat(MAX_QUERY_LENGTH + 1)).is_err());
    }

    #[test]
    fn query_rejects_empty_options_and_control_characters() {
        for query in [
            "",
            "   ",
            "--force",
            "  -x",
            "git\nvlc",
            "git\0",
            "tab\there",
        ] {
            assert!(checked_query(query).is_err(), "{query:?}");
        }
    }

    #[test]
    fn powershell_text_spells_quotes_as_code_units() {
        assert_eq!(
            powershell_text("a'\"$b"),
            "[string]::new([char[]](97,39,34,36,98))"
        );
        assert_eq!(
            powershell_text("é😀"),
            "[string]::new([char[]](233,55357,56832))"
        );
    }

    #[test]
    fn powershell_search_embeds_the_encoded_query_and_limit() {
        let script = powershell_search("x'; calc");
        assert!(script.contains("-Query ([string]::new([char[]](120,39,59,32,99,97,108,99)))"));
        assert!(script.contains("-Count 50"));
        assert!(!script.contains("calc"));
    }

    #[test]
    fn search_arguments_are_limited_and_non_interactive() {
        let args = search_args("vlc");
        assert!(args.windows(2).any(|pair| pair == ["--query", "vlc"]));
        assert!(args.windows(2).any(|pair| pair == ["--source", "winget"]));
        assert!(args.windows(2).any(|pair| pair == ["--count", "50"]));
        assert!(args.contains(&"--disable-interactivity"));
    }

    #[test]
    fn install_arguments_are_exact_silent_and_accept_agreements() {
        let args = install_args("Vendor.App");
        assert_eq!(args[0], "install");
        assert!(args.windows(2).any(|pair| pair == ["--id", "Vendor.App"]));
        assert!(args.windows(2).any(|pair| pair == ["--source", "winget"]));
        for flag in [
            "--exact",
            "--silent",
            "--accept-package-agreements",
            "--accept-source-agreements",
            "--disable-interactivity",
        ] {
            assert!(args.contains(&flag), "{flag}");
        }
    }

    #[test]
    fn install_rejects_invalid_ids_before_running_anything() {
        for id in ["", "--force", "Vendor.App --force", "Vendor.App\n"] {
            assert!(matches!(install(id), Err(AppError::Command(_))), "{id:?}");
        }
    }
}
