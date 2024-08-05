use std::{num::ParseIntError, process::Stdio};

use thiserror::Error;
use tokio::{io::AsyncWriteExt, process::Command};

#[derive(Debug)]
pub struct PdfInfo {
    pub title: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub author: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub mod_date: Option<String>,
    pub custom_metadata: Option<bool>,
    pub metadata_stream: Option<bool>,
    pub tagged: Option<bool>,
    pub user_properties: Option<bool>,
    pub suspects: Option<bool>,
    pub form: Option<String>,
    pub javascript: Option<bool>,
    pub pages: Option<usize>,
    pub encrypted: Option<bool>,
    pub page_size: Option<String>,
    pub page_rot: Option<String>,
    pub file_size: Option<String>,
    pub optimized: Option<bool>,
    pub pdf_version: Option<String>,
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

    #[error("file is not a pdf")]
    NotPdfFile,
}

pub async fn pdf_info(bytes: &[u8]) -> Result<PdfInfo, PdfInfoError> {
    let mut child = Command::new("pdfinfo")
        .args(["-"] /* PASS PDF THROUGH STDIN */)
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

        return Err(PdfInfoError::PdfInfoFailure(value.to_string()));
    }

    let value = String::from_utf8_lossy(&output.stdout);

    parse_pdf_info(&value)
}

fn parse_bool(value: &str) -> bool {
    value == "yes"
}

/// Parses the fields from the pdfinfo response
pub fn parse_pdf_info(output: &str) -> Result<PdfInfo, PdfInfoError> {
    let mut title = None;
    let mut subject = None;
    let mut keywords = None;
    let mut author = None;
    let mut producer = None;
    let mut creation_date = None;
    let mut mod_date = None;
    let mut custom_metadata = None;
    let mut metadata_stream = None;
    let mut tagged = None;
    let mut user_properties = None;
    let mut suspects = None;
    let mut form = None;
    let mut javascript = None;
    let mut pages = None;
    let mut encrypted = None;
    let mut page_size = None;
    let mut page_rot = None;
    let mut file_size = None;
    let mut optimized = None;
    let mut pdf_version = None;

    for line in output.lines() {
        let (key, value) = match line.split_once(':') {
            Some(value) => value,
            None => continue,
        };
        let value = value.trim_start();

        match key {
            "Title" => title = Some(value.to_string()),
            "Subject" => subject = Some(value.to_string()),
            "Keywords" => keywords = Some(value.to_string()),
            "Author" => author = Some(value.to_string()),
            "Producer" => producer = Some(value.to_string()),
            "CreationDate" => creation_date = Some(value.to_string()),
            "ModDate" => mod_date = Some(value.to_string()),
            "Custom Metadata" => custom_metadata = Some(parse_bool(value)),
            "Metadata Stream" => metadata_stream = Some(parse_bool(value)),
            "Tagged" => tagged = Some(parse_bool(value)),
            "UserProperties" => user_properties = Some(parse_bool(value)),
            "Suspects" => suspects = Some(parse_bool(value)),
            "Form" => form = Some(value.to_string()),
            "JavaScript" => javascript = Some(parse_bool(value)),
            "Pages" => {
                pages = Some(
                    value
                        .parse::<usize>()
                        .map_err(PdfInfoError::InvalidPageCount)?,
                )
            }
            "Encrypted" => encrypted = Some(parse_bool(value)),
            "Page size" => page_size = Some(value.to_string()),
            "Page rot" => page_rot = Some(value.to_string()),
            "File size" => file_size = Some(value.to_string()),
            "Optimized" => optimized = Some(parse_bool(value)),
            "PDF version" => pdf_version = Some(value.to_string()),
            _ => {}
        }
    }

    Ok(PdfInfo {
        title,
        subject,
        keywords,
        author,
        producer,
        creation_date,
        mod_date,
        custom_metadata,
        metadata_stream,
        tagged,
        user_properties,
        suspects,
        form,
        javascript,
        pages,
        encrypted,
        page_size,
        page_rot,
        file_size,
        optimized,
        pdf_version,
    })
}

#[cfg(test)]
mod test {
    use super::{parse_pdf_info, pdf_info};

    #[tokio::test]
    async fn test_invalid_file() {
        let value = &[b'A'];
        let err = pdf_info(value).await.unwrap_err();
        assert!(matches!(err, crate::info::PdfInfoError::NotPdfFile));
    }

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

        assert_eq!(
            output.title,
            Some("Ropes: an Alternative to Strings".to_string())
        );
        assert_eq!(output.subject, Some("".to_string()));
        assert_eq!(
            output.keywords,
            Some(
                "character strings, concatenation, Cedar, immutable, C, balanced trees".to_string()
            )
        );
        assert_eq!(
            output.author,
            Some("Hans-J. Boehm, Russ Atkinson and Michael Plass".to_string())
        );
        assert_eq!(
            output.producer,
            Some("Acrobat Distiller 2.0 for Windows".to_string())
        );
        assert_eq!(
            output.creation_date,
            Some("Sun Aug 25 21:00:20 1996 NZST".to_string())
        );
        assert_eq!(
            output.mod_date,
            Some("Sat Nov  2 06:49:17 1996 NZDT".to_string())
        );
        assert_eq!(output.custom_metadata, Some(false));
        assert_eq!(output.metadata_stream, Some(false));
        assert_eq!(output.tagged, Some(false));
        assert_eq!(output.user_properties, Some(false));
        assert_eq!(output.suspects, Some(false));
        assert_eq!(output.form, Some("none".to_string()));
        assert_eq!(output.javascript, Some(false));
        assert_eq!(output.pages, Some(16));
        assert_eq!(output.encrypted, Some(false));
        assert_eq!(output.page_size, Some("540 x 738 pts".to_string()));
        assert_eq!(output.page_rot, Some("0".to_string()));
        assert_eq!(output.file_size, Some("169205 bytes".to_string()));
        assert_eq!(output.optimized, Some(true));
        assert_eq!(output.pdf_version, Some("1.2".to_string()));
    }
}
