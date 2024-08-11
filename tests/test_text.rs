use pdf_process::{
    pdf_info, text_all_pages, text_all_pages_split, text_pages, text_single_page, Password,
    PdfInfoArgs, PdfTextArgs, PdfTextError, Secret,
};
use tokio::fs::read;

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

/// Tests preventing attempts extracting text on a page that goes out
/// of bounds from the acceptable number of pages
#[tokio::test]
async fn test_page_bounds() {
    let data = read("./tests/samples/test-pdf-2-pages.pdf").await.unwrap();

    let info = pdf_info(&data, &PdfInfoArgs::default()).await.unwrap();
    let args = PdfTextArgs::default();

    let err = text_single_page(&data, &info, 99, &args).await.unwrap_err();
    assert!(matches!(err, PdfTextError::PageOutOfBounds(99, 2)));

    let err = text_pages(&data, &info, vec![99], &args).await.unwrap_err();

    assert!(matches!(err, PdfTextError::PageOutOfBounds(99, 2)));
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

/// Tests reading when the file is encrypted and the incorrect password
/// is provided
#[tokio::test]
async fn test_encrypted_with_incorrect_password() {
    let data = read("./tests/samples/test-pdf-2-pages-encrypted.pdf")
        .await
        .unwrap();

    let info_args = PdfInfoArgs {
        password: Some(Password::User(Secret("password".to_string()))),
    };

    let info = pdf_info(&data, &info_args).await.unwrap();

    let args = PdfTextArgs {
        password: Some(Password::User(Secret("incorrect".to_string()))),
    };

    let err = text_all_pages(&data, &info, &args).await.unwrap_err();
    assert!(matches!(err, PdfTextError::IncorrectPassword));

    let err = text_single_page(&data, &info, 1, &args).await.unwrap_err();
    assert!(matches!(err, PdfTextError::IncorrectPassword));

    let err = text_all_pages_split(&data, &info, &args).await.unwrap_err();
    assert!(matches!(err, PdfTextError::IncorrectPassword));

    let err = text_pages(&data, &info, vec![1, 2], &args)
        .await
        .unwrap_err();
    assert!(matches!(err, PdfTextError::IncorrectPassword));
}
