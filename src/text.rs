use std::process::Stdio;

use thiserror::Error;
use tokio::{io::AsyncWriteExt, process::Command};

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

/// Renders a specific page from the pdf file
pub(crate) async fn pages_text<'data, 'options: 'data>(
    data: &'data [u8],
) -> Result<String, PdfTextError> {
    let mut child = Command::new("pdftotext")
        // Take input from stdin and provide to stdout
        .args(["-", "-"])
        // Pipe input and output for use
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(PdfTextError::SpawnProcess)?;

    // UNWRAP SAFETY: The child process is guaranteed to have a stdin as .stdin(Stdio::piped()) was called
    child
        .stdin
        .as_mut()
        .unwrap()
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

/// Renders a specific page from the pdf file
pub(crate) async fn page_text<'data, 'options: 'data>(
    data: &'data [u8],
    page: u32,
) -> Result<String, PdfTextError> {
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
        // Pipe input and output for use
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(PdfTextError::SpawnProcess)?;

    // UNWRAP SAFETY: The child process is guaranteed to have a stdin as .stdin(Stdio::piped()) was called
    child
        .stdin
        .as_mut()
        .unwrap()
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

    use crate::text::{page_text, pages_text};

    #[tokio::test]
    async fn test_all_content() {
        let expected = "Test pdf with text in it\n\n\u{c}";
        let data = read("./tests/samples/test-pdf.pdf").await.unwrap();
        let text = pages_text(&data).await.unwrap();
        assert_eq!(text.as_str(), expected);
    }

    #[tokio::test]
    async fn test_specific_page() {
        let data = read("./tests/samples/test-pdf-2-pages.pdf").await.unwrap();

        let expected = "Test pdf with text in it\n\n\u{c}";
        let text = page_text(&data, 1).await.unwrap();
        assert_eq!(text.as_str(), expected);

        let expected = "Test page 2\n\n\u{c}";
        let text = page_text(&data, 2).await.unwrap();
        assert_eq!(text.as_str(), expected);
    }
}
