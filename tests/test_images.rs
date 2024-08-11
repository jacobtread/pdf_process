use pdf_process::{
    pdf_info, render_all_pages, render_pages, render_single_page, OutputFormat, Password,
    PdfInfoArgs, PdfRenderError, RenderArgs,
};
use tokio::fs::read;

/// Tests rendering all pages
#[tokio::test]
async fn test_all_pages() {
    let data = read("./tests/samples/test-pdf-2-pages.pdf").await.unwrap();
    let info = pdf_info(&data, &PdfInfoArgs::default()).await.unwrap();
    let args = RenderArgs::default();
    let output = render_all_pages(&data, &info, OutputFormat::Jpeg, &args)
        .await
        .unwrap();

    assert_eq!(output.len(), 2);
}

/// Tests rendering a specific page
#[tokio::test]
async fn test_specific_page() {
    let data = read("./tests/samples/test-pdf-2-pages.pdf").await.unwrap();

    let info = pdf_info(&data, &PdfInfoArgs::default()).await.unwrap();
    let args = RenderArgs::default();

    let _output = render_single_page(&data, &info, OutputFormat::Jpeg, 1, &args)
        .await
        .unwrap();
}

/// Tests rendering a specific set of pages
#[tokio::test]
async fn test_specific_pages() {
    let data = read("./tests/samples/test-pdf-2-pages.pdf").await.unwrap();

    let info = pdf_info(&data, &PdfInfoArgs::default()).await.unwrap();
    let args = RenderArgs::default();

    let output = render_pages(&data, &info, OutputFormat::Jpeg, vec![1, 2], &args)
        .await
        .unwrap();

    assert_eq!(output.len(), 2);
}

/// Tests preventing attempts at rendering a page that goes out
/// of bounds from the acceptable number of pages
#[tokio::test]
async fn test_page_bounds() {
    let data = read("./tests/samples/test-pdf-2-pages.pdf").await.unwrap();

    let info = pdf_info(&data, &PdfInfoArgs::default()).await.unwrap();
    let args = RenderArgs::default();

    let err = render_single_page(&data, &info, OutputFormat::Jpeg, 99, &args)
        .await
        .unwrap_err();
    assert!(matches!(err, PdfRenderError::PageOutOfBounds(99, 2)));

    let err = render_pages(&data, &info, OutputFormat::Jpeg, vec![99], &args)
        .await
        .unwrap_err();

    assert!(matches!(err, PdfRenderError::PageOutOfBounds(99, 2)));
}

/// Tests prevents rendering when the pdf info specifies a password
/// but the render args didn't provide a password
#[tokio::test]
async fn test_encrypted() {
    let data = read("./tests/samples/test-pdf-2-pages-encrypted.pdf")
        .await
        .unwrap();

    let info_args = PdfInfoArgs::default().set_password(Password::user("password"));
    let info = pdf_info(&data, &info_args).await.unwrap();
    let args = RenderArgs::default();

    let err = render_single_page(&data, &info, OutputFormat::Jpeg, 99, &args)
        .await
        .unwrap_err();
    assert!(matches!(err, PdfRenderError::PdfEncrypted));

    let err = render_pages(&data, &info, OutputFormat::Jpeg, vec![99], &args)
        .await
        .unwrap_err();

    assert!(matches!(err, PdfRenderError::PdfEncrypted));
}

/// Tests rendering an encrypted pdf when the password is provided
#[tokio::test]
async fn test_encrypted_with_password() {
    let data = read("./tests/samples/test-pdf-2-pages-encrypted.pdf")
        .await
        .unwrap();

    let info_args = PdfInfoArgs::default().set_password(Password::user("password"));
    let info = pdf_info(&data, &info_args).await.unwrap();
    let args = RenderArgs::default().set_password(Password::user("password"));

    let _output = render_single_page(&data, &info, OutputFormat::Jpeg, 2, &args)
        .await
        .unwrap();

    let output = render_all_pages(&data, &info, OutputFormat::Jpeg, &args)
        .await
        .unwrap();

    assert_eq!(output.len(), 2);

    let output = render_pages(&data, &info, OutputFormat::Jpeg, vec![1, 2], &args)
        .await
        .unwrap();

    assert_eq!(output.len(), 2);
}

/// Tests rendering an encrypted pdf when the password is provided
/// but incorrect
#[tokio::test]
async fn test_encrypted_with_incorrect_password() {
    let data = read("./tests/samples/test-pdf-2-pages-encrypted.pdf")
        .await
        .unwrap();

    let info_args = PdfInfoArgs::default().set_password(Password::user("password"));

    let info = pdf_info(&data, &info_args).await.unwrap();
    let args = RenderArgs::default().set_password(Password::user("incorrect"));

    let err = render_single_page(&data, &info, OutputFormat::Jpeg, 1, &args)
        .await
        .unwrap_err();
    assert!(matches!(err, PdfRenderError::IncorrectPassword));

    let err = render_pages(&data, &info, OutputFormat::Jpeg, vec![1], &args)
        .await
        .unwrap_err();

    assert!(matches!(err, PdfRenderError::IncorrectPassword));
}
