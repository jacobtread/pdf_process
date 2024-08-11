use std::process::Stdio;

use thiserror::Error;
use tokio::{io::AsyncWriteExt, process::Command};

use crate::shared::Password;

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
    let args = args.build_args();
    let mut child = Command::new("pdftotext")
        // Take input from stdin and provide to stdout
        .args(["-", "-"])
        .args(args)
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
    let args = args.build_args();
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
        .args(args)
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

        return Err(PdfTextError::PdfTextFailure(value.to_string()));
    }

    let value = String::from_utf8_lossy(&output.stdout);

    Ok(value.into_owned())
}

#[cfg(test)]
mod test {
    use tokio::fs::read;

    use crate::text::{page_text, pages_text, PdfTextArgs};

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

        let expected = "Test pdf with text in it\n\n\u{c}";
        let text = page_text(&data, 1, &PdfTextArgs::default()).await.unwrap();
        assert_eq!(text.as_str(), expected);

        let expected = "Test page 2\n\n\u{c}";
        let text = page_text(&data, 2, &PdfTextArgs::default()).await.unwrap();
        assert_eq!(text.as_str(), expected);
    }
}
