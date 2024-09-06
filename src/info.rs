//! Helpers getting info about PDF files
//!  
//! * [pdf_info] - Get info from a PDF file

use std::{collections::HashMap, num::ParseIntError, process::Stdio};

use thiserror::Error;
use tokio::{io::AsyncWriteExt, process::Command};

use crate::shared::Password;

/// Pdf file may be "encrypted" but still readable
#[derive(Debug)]
pub struct PdfInfoEncryption {
    /// Whether encryption is enabled
    encrypted: bool,
    /// Encryption options that are set
    options: HashMap<String, String>,
}

impl PdfInfoEncryption {
    pub fn is_encrypted(&self) -> bool {
        self.encrypted
    }

    pub fn is_print_allowed(&self) -> bool {
        let value = match self.options.get("print") {
            Some(value) => value,
            None => return true,
        };

        value == "yes"
    }

    pub fn is_copy_allowed(&self) -> bool {
        let value = match self.options.get("copy") {
            Some(value) => value,
            None => return true,
        };

        value == "yes"
    }

    pub fn is_change_allowed(&self) -> bool {
        let value = match self.options.get("change") {
            Some(value) => value,
            None => return true,
        };

        value == "yes"
    }

    pub fn is_add_notes_allowed(&self) -> bool {
        let value = match self.options.get("addNotes") {
            Some(value) => value,
            None => return true,
        };

        value == "yes"
    }
    pub fn algorithm(&self) -> Option<&str> {
        self.options.get("algorithm").map(|value| value.as_str())
    }
}

/// Parses the fields from the pdfinfo response
fn parse_pdf_info_encryption(output: &str) -> Result<PdfInfoEncryption, PdfInfoError> {
    let (encrypted, options) = output
        .split_once(' ')
        .ok_or(PdfInfoError::MalformedEncryptionOptions)?;
    let encrypted = parse_bool(encrypted);

    // Strip the braces
    let options = options
        .strip_prefix('(')
        .ok_or(PdfInfoError::MalformedEncryptionOptions)?;
    let options = options
        .strip_suffix(')')
        .ok_or(PdfInfoError::MalformedEncryptionOptions)?;

    let parts = options.split_whitespace();

    let options = parts
        .filter_map(|value| {
            let (key, value) = value.split_once(':')?;
            let value = value.trim_start();
            Some((key.to_string(), value.to_string()))
        })
        .collect();

    // yes (print:yes copy:no change:no addNotes:no algorithm:AES-256)

    Ok(PdfInfoEncryption { encrypted, options })
}

#[derive(Debug)]
pub struct PdfInfo {
    /// Data parsed from the pdfinfo cli
    data: HashMap<String, String>,
}

impl PdfInfo {
    fn data(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(String::as_str)
    }

    pub fn pages(&self) -> Option<Result<u32, ParseIntError>> {
        self.data("Pages").map(|value| value.parse::<u32>())
    }

    pub fn title(&self) -> Option<&str> {
        self.data("Title")
    }

    pub fn subject(&self) -> Option<&str> {
        self.data("Subject")
    }

    pub fn keywords(&self) -> Option<&str> {
        self.data("Keywords")
    }

    pub fn creator(&self) -> Option<&str> {
        self.data("Creator")
    }

    pub fn producer(&self) -> Option<&str> {
        self.data("Producer")
    }

    pub fn creation_date(&self) -> Option<&str> {
        self.data("CreationDate")
    }

    pub fn mod_date(&self) -> Option<&str> {
        self.data("ModDate")
    }

    pub fn author(&self) -> Option<&str> {
        self.data("Author")
    }

    pub fn custom_metadata(&self) -> Option<bool> {
        self.data("Custom Metadata").map(parse_bool)
    }

    pub fn metadata_stream(&self) -> Option<bool> {
        self.data("Metadata Stream").map(parse_bool)
    }

    pub fn tagged(&self) -> Option<bool> {
        self.data("Tagged").map(parse_bool)
    }

    pub fn user_properties(&self) -> Option<bool> {
        self.data("UserProperties").map(parse_bool)
    }

    pub fn suspects(&self) -> Option<bool> {
        self.data("Suspects").map(parse_bool)
    }

    pub fn form(&self) -> Option<&str> {
        self.data("Form")
    }

    pub fn page_size(&self) -> Option<&str> {
        self.data("Page size")
    }

    pub fn javascript(&self) -> Option<bool> {
        self.data("JavaScript").map(parse_bool)
    }

    pub fn encrypted(&self) -> Option<bool> {
        self.data("Encrypted").map(|value| value.starts_with("yes"))
    }

    pub fn encryption_raw(&self) -> Option<&str> {
        self.data("Encrypted")
    }

    /// Obtains the encryption details from the file
    pub fn encryption(&self) -> Option<Result<PdfInfoEncryption, PdfInfoError>> {
        self.data("Encrypted").map(parse_pdf_info_encryption)
    }

    pub fn page_rot(&self) -> Option<&str> {
        self.data("Page rot")
    }

    pub fn file_size(&self) -> Option<&str> {
        self.data("File size")
    }

    pub fn optimized(&self) -> Option<bool> {
        self.data("Optimized").map(parse_bool)
    }

    pub fn pdf_version(&self) -> Option<&str> {
        self.data("PDF version")
    }
}

#[derive(Debug, Error)]
pub enum PdfInfoError {
    #[error("failed to spawn pdfinfo: {0}")]
    SpawnProcess(std::io::Error),
    #[error("failed to write pdf bytes: {0}")]
    WritePdf(std::io::Error),

    #[error("failed to get output: {0}")]
    WaitOutput(std::io::Error),

    #[error("invalid page count: {0}")]
    InvalidPageCount(ParseIntError),

    #[error("failed to get pdfinfo exit code: {0}")]
    PdfInfoFailure(String),

    #[error("pdf file is encrypted")]
    PdfEncrypted,

    #[error("incorrect password was provided")]
    IncorrectPassword,

    #[error("file is not a pdf")]
    NotPdfFile,

    #[error("encryption options are malformed")]
    MalformedEncryptionOptions,
}

#[derive(Debug, Default, Clone)]
pub struct PdfInfoArgs {
    /// Password for the PDF
    pub password: Option<Password>,
}

impl PdfInfoArgs {
    pub fn set_password(mut self, password: Password) -> Self {
        self.password = Some(password);
        self
    }

    /// Builds an argument list from all the options
    pub fn build_args(&self) -> Vec<String> {
        let mut out = Vec::new();

        if let Some(password) = self.password.as_ref() {
            password.push_arg(&mut out);
        }

        out
    }
}

/// Extracts information about the provided PDF file
///
/// ## Arguments
/// * data - The raw PDF file bytes
/// * args - Extra args to provide to pdfinfo
pub async fn pdf_info(bytes: &[u8], args: &PdfInfoArgs) -> Result<PdfInfo, PdfInfoError> {
    let cli_args = args.build_args();

    let mut child = Command::new("pdfinfo")
        .args(["-"] /* PASS PDF THROUGH STDIN */)
        .args(cli_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(PdfInfoError::SpawnProcess)?;

    // UNWRAP SAFETY: The child process is guaranteed to have a stdin as .stdin(Stdio::piped()) was called
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(bytes)
        .await
        .map_err(PdfInfoError::WritePdf)?;

    let output = child
        .wait_with_output()
        .await
        .map_err(PdfInfoError::WaitOutput)?;

    // Handle info failure
    if !output.status.success() {
        let value = String::from_utf8_lossy(&output.stderr);

        if value.contains("May not be a PDF file") {
            return Err(PdfInfoError::NotPdfFile);
        }

        if value.contains("Incorrect password") {
            return Err(if args.password.is_none() {
                PdfInfoError::PdfEncrypted
            } else {
                PdfInfoError::IncorrectPassword
            });
        }

        return Err(PdfInfoError::PdfInfoFailure(value.to_string()));
    }

    let value = String::from_utf8_lossy(&output.stdout);

    parse_pdf_info(&value)
}

fn parse_bool(value: &str) -> bool {
    value == "yes"
}

/// Parses the fields from the pdfinfo response
fn parse_pdf_info(output: &str) -> Result<PdfInfo, PdfInfoError> {
    let data = output
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            let value = value.trim_start();
            Some((key.to_string(), value.to_string()))
        })
        .collect();

    Ok(PdfInfo { data })
}

#[cfg(test)]
mod test {
    use super::{parse_pdf_info, pdf_info, PdfInfoArgs};

    /// Tests against an invalid file
    #[tokio::test]
    async fn test_invalid_file() {
        let value = &[b'A'];
        let err = pdf_info(value, &PdfInfoArgs::default()).await.unwrap_err();
        assert!(matches!(err, crate::info::PdfInfoError::NotPdfFile));
    }

    /// Tests the output parser logic
    #[test]
    fn test_parsing_output() {
        let value = r#"
Title:           Ropes: an Alternative to Strings
Subject:         
Keywords:        character strings, concatenation, Cedar, immutable, C, balanced trees
Author:          Hans-J. Boehm, Russ Atkinson and Michael Plass
Producer:        Acrobat Distiller 2.0 for Windows
CreationDate:    Sun Aug 25 21:00:20 1996 NZST
ModDate:         Sat Nov  2 06:49:17 1996 NZDT
Custom Metadata: no
Metadata Stream: no
Tagged:          no
UserProperties:  no
Suspects:        no
Form:            none
JavaScript:      no
Pages:           16
Encrypted:       no
Page size:       540 x 738 pts
Page rot:        0
File size:       169205 bytes
Optimized:       yes
PDF version:     1.2
        "#;
        let output = parse_pdf_info(value).unwrap();

        assert_eq!(output.title(), Some("Ropes: an Alternative to Strings"));
        assert_eq!(output.subject(), Some(""));
        assert_eq!(
            output.keywords(),
            Some("character strings, concatenation, Cedar, immutable, C, balanced trees")
        );
        assert_eq!(
            output.author(),
            Some("Hans-J. Boehm, Russ Atkinson and Michael Plass")
        );
        assert_eq!(output.producer(), Some("Acrobat Distiller 2.0 for Windows"));
        assert_eq!(
            output.creation_date(),
            Some("Sun Aug 25 21:00:20 1996 NZST")
        );
        assert_eq!(output.mod_date(), Some("Sat Nov  2 06:49:17 1996 NZDT"));
        assert_eq!(output.custom_metadata(), Some(false));
        assert_eq!(output.metadata_stream(), Some(false));
        assert_eq!(output.tagged(), Some(false));
        assert_eq!(output.user_properties(), Some(false));
        assert_eq!(output.suspects(), Some(false));
        assert_eq!(output.form(), Some("none"));
        assert_eq!(output.javascript(), Some(false));
        assert_eq!(output.pages(), Some(Ok(16)));
        assert_eq!(output.encrypted(), Some(false));
        assert_eq!(output.page_size(), Some("540 x 738 pts"));
        assert_eq!(output.page_rot(), Some("0"));
        assert_eq!(output.file_size(), Some("169205 bytes"));
        assert_eq!(output.optimized(), Some(true));
        assert_eq!(output.pdf_version(), Some("1.2"));
    }
}
