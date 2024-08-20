//! Helpers for rendering images from PDF files
//!  
//! * [text_all_pages] - Gets the text from all pages as a single string
//! * [text_all_pages_split] - Gets the text from all pages as separate strings
//! * [text_pages] - Gets the text from a specific set of pages as separate strings
//! * [text_single_page] - Gets the text from a specific page

use futures_util::{stream::FuturesOrdered, TryStreamExt};
use std::process::Stdio;
use thiserror::Error;
use tokio::{io::AsyncWriteExt, process::Command};

use crate::{info::PdfInfo, shared::Password};

/// Character that indicates the end of a page in a PDF file
pub const PAGE_END_CHARACTER: char = '\u{c}';

#[derive(Debug, Error)]
pub enum PdfTextError {
    #[error("failed to spawn pdftotext: {0}")]
    SpawnProcess(std::io::Error),

    #[error("failed to write pdf bytes: {0}")]
    WritePdf(std::io::Error),

    #[error("failed to get output: {0}")]
    WaitOutput(std::io::Error),

    #[error("failed to get pdfinfo exit code: {0}")]
    PdfTextFailure(String),

    #[error("page {0} is outside the number of available pages {1}")]
    PageOutOfBounds(u32, u32),

    #[error("page info page count is missing or invalid, pdf likely invalid")]
    PageCountUnknown,

    #[error("pdf is encrypted and no password was provided")]
    PdfEncrypted,

    #[error("incorrect password was provided")]
    IncorrectPassword,

    #[error("file is not a pdf")]
    NotPdfFile,
}

#[derive(Debug, Default, Clone)]
pub struct PdfTextArgs {
    /// Password for the PDF
    pub password: Option<Password>,
}

impl PdfTextArgs {
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

/// Extracts the text from all the pages in the provided PDF.
/// Replaces the page break characters with a single new line
/// provides all pages as a single string.
///
/// Use [text_all_pages_split] to get a separate string for
/// each page as a list
///
/// If you only want a specific page use [text_single_page]
///
/// ## Arguments
/// * data - The raw PDF file bytes
/// * info - The PDF info to use for the page count and encryption state
/// * args - Optional args for the pdf to text
pub async fn text_all_pages(data: &[u8], args: &PdfTextArgs) -> Result<String, PdfTextError> {
    let value = pages_text(data, args).await?;

    // Strip page end characters
    let value = value.replace(PAGE_END_CHARACTER, "\n");

    Ok(value)
}

/// Extracts the text from all the pages in the provided PDF.
/// Provides a list of strings one string per page. Pages are
/// split on the [PAGE_END_CHARACTER]
///
/// If you only want a specific page use [text_single_page]
///
///
/// ## Arguments
/// * data - The raw PDF file bytes
/// * info - The PDF info to use for the page count and encryption state
/// * args - Optional args for the pdf to text
pub async fn text_all_pages_split(
    data: &[u8],
    args: &PdfTextArgs,
) -> Result<Vec<String>, PdfTextError> {
    let out = pages_text(data, args).await?;

    // Split on page ends
    Ok(out
        .split(PAGE_END_CHARACTER)
        .map(|value| value.to_string())
        .collect())
}

/// Extracts the text from the  provided pages in the provided PDF.
/// Provides a list of strings one string per page. Pages are
/// split on the [PAGE_END_CHARACTER]
///
/// If you only want a specific page use [text_single_page]
///
/// ## Arguments
/// * data - The raw PDF file bytes
/// * info - The PDF info to use for the page count and encryption state
/// * pages - The page numbers to get text from
/// * args - Optional args for the pdf to text
pub async fn text_pages(
    data: &[u8],
    info: &PdfInfo,
    pages: Vec<u32>,
    args: &PdfTextArgs,
) -> Result<Vec<String>, PdfTextError> {
    // Get the page count
    let page_count = info
        .pages()
        .ok_or(PdfTextError::PageCountUnknown)?
        .map_err(|_| PdfTextError::PageCountUnknown)?;

    // Validate requested pages
    for page in &pages {
        if *page > page_count {
            return Err(PdfTextError::PageOutOfBounds(*page, page_count));
        }
    }
    // Render all the pages individually
    pages
        .into_iter()
        .map(|page| page_text(data, page, args))
        .collect::<FuturesOrdered<_>>()
        .try_collect()
        .await
}

/// Extracts the text from the specific pages in the provided PDF.
///
/// ## Arguments
/// * data - The raw PDF file bytes
/// * info - The PDF info to use for the page count and encryption state
/// * page - The page number to get text from
/// * args - Optional args for the pdf to text
pub async fn text_single_page(
    data: &[u8],
    info: &PdfInfo,
    page: u32,
    args: &PdfTextArgs,
) -> Result<String, PdfTextError> {
    // Get the page count
    let page_count = info
        .pages()
        .ok_or(PdfTextError::PageCountUnknown)?
        .map_err(|_| PdfTextError::PageCountUnknown)?;

    // Validate chosen page
    if page > page_count {
        return Err(PdfTextError::PageOutOfBounds(page, page_count));
    }

    page_text(data, page, args).await
}

/// Extracts the text contents from the provided pdf file data
/// using the `pdftotext` program.
///
/// Extracts the text from all the pages into a single string
/// use [page_text] to extract the text for a single page
///
/// INTERNAL USE ONLY: Does not validate that the page is within the
/// valid page bounds use one of the other functions above
///
/// ## Arguments
/// * data - The raw PDF file bytes
/// * args - Extra args to provide to pdftotext
async fn pages_text(data: &[u8], args: &PdfTextArgs) -> Result<String, PdfTextError> {
    let cli_args = args.build_args();
    let mut child = Command::new("pdftotext")
        // Take input from stdin and provide to stdout
        .args(["-", "-"])
        .args(cli_args)
        // Pipe input and output for use
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(PdfTextError::SpawnProcess)?;

    child
        .stdin
        .as_mut()
        // Should always have stdin when using .stdin(Stdio::piped())
        .expect("progress missing stdin after being piped")
        .write_all(data)
        .await
        .map_err(PdfTextError::WritePdf)?;

    let output = child
        .wait_with_output()
        .await
        .map_err(PdfTextError::WaitOutput)?;

    // Handle info failure
    if !output.status.success() {
        let value = String::from_utf8_lossy(&output.stderr);

        if value.contains("May not be a PDF file") {
            return Err(PdfTextError::NotPdfFile);
        }

        if value.contains("Incorrect password") {
            return Err(if args.password.is_none() {
                PdfTextError::PdfEncrypted
            } else {
                PdfTextError::IncorrectPassword
            });
        }

        return Err(PdfTextError::PdfTextFailure(value.to_string()));
    }

    let value = String::from_utf8_lossy(&output.stdout);
    Ok(value.into_owned())
}

/// Extracts the text contents from the provided pdf file data
/// using the `pdftotext` program
///
/// INTERNAL USE ONLY: Does not validate that the page is within the
/// valid page bounds use one of the other functions above
///
/// ## Arguments
/// * data - The raw PDF file
/// * page - The page to extract text from
/// * args - Extra args to provide to pdftotext
async fn page_text(data: &[u8], page: u32, args: &PdfTextArgs) -> Result<String, PdfTextError> {
    let cli_args = args.build_args();
    let mut child = Command::new("pdftotext")
        // Take input from stdin and provide to stdout
        .args(["-", "-"])
        // Add the page args
        .args([
            "-f".to_string(),
            format!("{page}"),
            "-l".to_string(),
            format!("{page}"),
        ])
        .args(cli_args)
        // Pipe input and output for use
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(PdfTextError::SpawnProcess)?;

    child
        .stdin
        .as_mut()
        // Should always have stdin when using .stdin(Stdio::piped())
        .expect("progress missing stdin after being piped")
        .write_all(data)
        .await
        .map_err(PdfTextError::WritePdf)?;

    let output = child
        .wait_with_output()
        .await
        .map_err(PdfTextError::WaitOutput)?;

    // Handle info failure
    if !output.status.success() {
        let value = String::from_utf8_lossy(&output.stderr);

        if value.contains("May not be a PDF file") {
            return Err(PdfTextError::NotPdfFile);
        }

        if value.contains("Incorrect password") {
            return Err(if args.password.is_none() {
                PdfTextError::PdfEncrypted
            } else {
                PdfTextError::IncorrectPassword
            });
        }

        return Err(PdfTextError::PdfTextFailure(value.to_string()));
    }

    let value = String::from_utf8_lossy(&output.stdout);
    let mut value = value.to_string();

    // Strip the page end char
    if value.ends_with(PAGE_END_CHARACTER) {
        value.pop();
    }

    Ok(value)
}

#[cfg(test)]
mod test {
    use crate::text::{page_text, pages_text, PdfTextArgs, PdfTextError};
    use tokio::fs::read;

    /// Tests invalid files are handled
    #[tokio::test]
    async fn test_invalid_file() {
        let err = pages_text(&[b'A'], &PdfTextArgs::default())
            .await
            .unwrap_err();
        assert!(matches!(err, PdfTextError::NotPdfFile));
    }

    /// Tests reading text from all pages
    #[tokio::test]
    async fn test_all_content() {
        let expected = "Test pdf with text in it\n\n\u{c}";
        let data = read("./tests/samples/test-pdf.pdf").await.unwrap();
        let text = pages_text(&data, &PdfTextArgs::default()).await.unwrap();
        assert_eq!(text.as_str(), expected);
    }

    /// Tests reading the text from a specific page
    #[tokio::test]
    async fn test_specific_page() {
        let data = read("./tests/samples/test-pdf-2-pages.pdf").await.unwrap();

        let expected = "Test pdf with text in it\n\n";
        let text = page_text(&data, 1, &PdfTextArgs::default()).await.unwrap();
        assert_eq!(text.as_str(), expected);

        let expected = "Test page 2\n\n";
        let text = page_text(&data, 2, &PdfTextArgs::default()).await.unwrap();
        assert_eq!(text.as_str(), expected);
    }
}
