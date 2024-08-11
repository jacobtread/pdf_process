use std::process::Stdio;

use futures::{stream::FuturesOrdered, TryStreamExt};
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
    /// Builds an argument list from all the options
    pub fn build_args(&self) -> Vec<String> {
        let mut out = Vec::new();

        if let Some(password) = self.password.as_ref() {
            password.push_arg(&mut out);
        }

        out
    }
}

pub async fn text_all_pages(
    data: &[u8],
    info: &PdfInfo,
    args: &PdfTextArgs,
) -> Result<String, PdfTextError> {
    // Check encryption
    if info.encrypted().unwrap_or_default() && args.password.is_none() {
        return Err(PdfTextError::PdfEncrypted);
    }

    let value = pages_text(data, args).await?;

    // Strip page end characters
    let value = value.replace(PAGE_END_CHARACTER, "\n");

    Ok(value)
}

pub async fn text_all_pages_split(
    data: &[u8],
    info: &PdfInfo,
    args: &PdfTextArgs,
) -> Result<Vec<String>, PdfTextError> {
    // Check encryption
    if info.encrypted().unwrap_or_default() && args.password.is_none() {
        return Err(PdfTextError::PdfEncrypted);
    }

    let out = pages_text(data, args).await?;

    // Split on page ends
    Ok(out
        .split(PAGE_END_CHARACTER)
        .map(|value| value.to_string())
        .collect())
}

pub async fn text_pages(
    data: &[u8],
    info: &PdfInfo,
    pages: Vec<u32>,
    args: &PdfTextArgs,
) -> Result<Vec<String>, PdfTextError> {
    // Check encryption
    if info.encrypted().unwrap_or_default() && args.password.is_none() {
        return Err(PdfTextError::PdfEncrypted);
    }

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

pub async fn text_single_page(
    data: &[u8],
    info: &PdfInfo,
    page: u32,
    args: &PdfTextArgs,
) -> Result<String, PdfTextError> {
    // Check encryption
    if info.encrypted().unwrap_or_default() && args.password.is_none() {
        return Err(PdfTextError::PdfEncrypted);
    }

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
/// ## Arguments
/// * data - The raw PDF file bytes
/// * args - Extra args to provide to pdftotext
pub(crate) async fn pages_text(data: &[u8], args: &PdfTextArgs) -> Result<String, PdfTextError> {
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
/// ## Arguments
/// * data - The raw PDF file
/// * page - The page to extract text from
/// * args - Extra args to provide to pdftotext
pub(crate) async fn page_text(
    data: &[u8],
    page: u32,
    args: &PdfTextArgs,
) -> Result<String, PdfTextError> {
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
    use tokio::fs::read;

    use crate::{
        info::{pdf_info, PdfInfoArgs},
        shared::{Password, Secret},
        text::{
            page_text, pages_text, text_all_pages, text_all_pages_split, text_pages,
            text_single_page, PdfTextArgs, PdfTextError,
        },
    };

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

    /// Tests reading the text from a specific page
    #[tokio::test]
    async fn test_single_page() {
        let data = read("./tests/samples/test-pdf-2-pages.pdf").await.unwrap();

        let info = pdf_info(&data, &PdfInfoArgs::default()).await.unwrap();

        let expected = "Test pdf with text in it\n\n";
        let text = text_single_page(&data, &info, 1, &PdfTextArgs::default())
            .await
            .unwrap();
        assert_eq!(text.as_str(), expected);

        let expected = "Test page 2\n\n";
        let text = text_single_page(&data, &info, 2, &PdfTextArgs::default())
            .await
            .unwrap();
        assert_eq!(text.as_str(), expected);
    }

    /// Tests reading the text from all pages
    #[tokio::test]
    async fn test_all_pages() {
        let data = read("./tests/samples/test-pdf-2-pages.pdf").await.unwrap();

        let info = pdf_info(&data, &PdfInfoArgs::default()).await.unwrap();

        let expected = "Test pdf with text in it\n\n\nTest page 2\n\n\n";
        let text = text_all_pages(&data, &info, &PdfTextArgs::default())
            .await
            .unwrap();
        assert_eq!(text.as_str(), expected);
    }

    /// Tests reading specific pages text
    #[tokio::test]
    async fn test_pages() {
        let data = read("./tests/samples/test-pdf-2-pages.pdf").await.unwrap();

        let info = pdf_info(&data, &PdfInfoArgs::default()).await.unwrap();

        let expected = vec![
            "Test pdf with text in it\n\n".to_string(),
            "Test page 2\n\n".to_string(),
        ];
        let text = text_pages(&data, &info, vec![1, 2], &PdfTextArgs::default())
            .await
            .unwrap();
        assert_eq!(text, expected);
    }

    /// Tests reading all pages text in split form
    #[tokio::test]
    async fn test_all_pages_split() {
        let data = read("./tests/samples/test-pdf-2-pages.pdf").await.unwrap();

        let info = pdf_info(&data, &PdfInfoArgs::default()).await.unwrap();

        let expected = vec![
            "Test pdf with text in it\n\n".to_string(),
            "Test page 2\n\n".to_string(),
            "".to_string(),
        ];
        let text = text_all_pages_split(&data, &info, &PdfTextArgs::default())
            .await
            .unwrap();
        assert_eq!(text, expected);
    }

    /// Tests reading when the file is encrypted
    #[tokio::test]
    async fn test_encrypted() {
        let data = read("./tests/samples/test-pdf-2-pages-encrypted.pdf")
            .await
            .unwrap();

        let info_args = PdfInfoArgs {
            password: Some(Password::User(Secret("password".to_string()))),
        };

        let info = pdf_info(&data, &info_args).await.unwrap();

        let err = text_all_pages(&data, &info, &PdfTextArgs::default())
            .await
            .unwrap_err();
        assert!(matches!(err, PdfTextError::PdfEncrypted));

        let err = text_single_page(&data, &info, 1, &PdfTextArgs::default())
            .await
            .unwrap_err();
        assert!(matches!(err, PdfTextError::PdfEncrypted));

        let err = text_all_pages_split(&data, &info, &PdfTextArgs::default())
            .await
            .unwrap_err();
        assert!(matches!(err, PdfTextError::PdfEncrypted));

        let err = text_pages(&data, &info, vec![1, 2], &PdfTextArgs::default())
            .await
            .unwrap_err();
        assert!(matches!(err, PdfTextError::PdfEncrypted));
    }

    /// Tests reading when the file is encrypted but the password
    /// is provided
    #[tokio::test]
    async fn test_encrypted_with_password() {
        let data = read("./tests/samples/test-pdf-2-pages-encrypted.pdf")
            .await
            .unwrap();

        let info_args = PdfInfoArgs {
            password: Some(Password::User(Secret("password".to_string()))),
        };

        let info = pdf_info(&data, &info_args).await.unwrap();

        let args = PdfTextArgs {
            password: Some(Password::User(Secret("password".to_string()))),
        };

        text_all_pages(&data, &info, &args).await.unwrap();
        text_single_page(&data, &info, 1, &args).await.unwrap();
        text_all_pages_split(&data, &info, &args).await.unwrap();
        text_pages(&data, &info, vec![1, 2], &args).await.unwrap();
    }
}
