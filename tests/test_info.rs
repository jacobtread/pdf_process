use pdf_process::{pdf_info, Password, PdfInfoArgs, PdfInfoError, Secret};
use tokio::fs::read;

/// Tests from actual files
#[tokio::test]
async fn test_actual_files() {
    let data = read("./tests/samples/test-pdf-2-pages.pdf").await.unwrap();
    let info = pdf_info(&data, &PdfInfoArgs::default()).await.unwrap();
    assert_eq!(info.pages(), Some(Ok(2)));

    let data = read("./tests/samples/test-pdf.pdf").await.unwrap();
    let info = pdf_info(&data, &PdfInfoArgs::default()).await.unwrap();
    assert_eq!(info.pages(), Some(Ok(1)));
}

/// Tests getting pdfinfo from an encrypted file when the password is not set
#[tokio::test]
async fn test_encrypted() {
    let data = read("./tests/samples/test-pdf-2-pages-encrypted.pdf")
        .await
        .unwrap();

    let err = pdf_info(&data, &PdfInfoArgs::default()).await.unwrap_err();

    assert!(matches!(err, PdfInfoError::PdfEncrypted));
}

/// Tests getting pdfinfo from a encrypted file when the password is set
#[tokio::test]
async fn test_encrypted_with_password() {
    let data = read("./tests/samples/test-pdf-2-pages-encrypted.pdf")
        .await
        .unwrap();
    let args = PdfInfoArgs {
        password: Some(Password::Owner(Secret("password".to_string()))),
    };
    let info = pdf_info(&data, &args).await.unwrap();

    assert_eq!(info.pages(), Some(Ok(2)));
    assert_eq!(info.encrypted(), Some(true));

    let args = PdfInfoArgs {
        password: Some(Password::User(Secret("password".to_string()))),
    };
    let info = pdf_info(&data, &args).await.unwrap();

    assert_eq!(info.pages(), Some(Ok(2)));
    assert_eq!(info.encrypted(), Some(true));
}

/// Tests getting pdfinfo from a encrypted file when the password is set
#[tokio::test]
async fn test_encrypted_with_incorrect_password() {
    let data = read("./tests/samples/test-pdf-2-pages-encrypted.pdf")
        .await
        .unwrap();
    let args = PdfInfoArgs {
        password: Some(Password::User(Secret("incorrect".to_string()))),
    };
    let err = pdf_info(&data, &args).await.unwrap_err();

    assert!(matches!(err, PdfInfoError::IncorrectPassword));
}
