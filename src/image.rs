//! Helpers for rendering images from PDF files
//!  
//! * [render_all_pages] - Renders all pages in the PDF file
//! * [render_pages] - Renders a specific set of pages
//! * [render_single_page] - Renders a specific page

use std::process::Stdio;

use futures::{stream::FuturesOrdered, TryStreamExt};
use image::{DynamicImage, ImageError, ImageFormat};
use thiserror::Error;
use tokio::{io::AsyncWriteExt, process::Command};

use crate::{info::PdfInfo, shared::Password};

#[derive(Default)]
pub struct RenderArgs {
    /// Optional custom resolution to render at, defaults to 150 PPI
    pub resolution: Option<Resolution>,
    /// Optionally scale to a specific size
    pub scale_to: Option<ScaleTo>,

    /// Area to render
    pub render_area: Option<RenderArea>,
    /// Rendered page content colors
    pub render_color: Option<RenderColor>,
    /// Rendered page color
    pub page_color: Option<PageColor>,

    /// Password for the PDF
    pub password: Option<Password>,
}

impl RenderArgs {
    /// Builds an argument list from all the options
    pub fn build_args(&self) -> Vec<String> {
        let mut out = Vec::new();

        if let Some(res) = self.resolution.as_ref() {
            res.push_arg(&mut out);
        }

        if let Some(scale_to) = self.scale_to.as_ref() {
            scale_to.push_arg(&mut out);
        }

        if let Some(render_area) = self.render_area.as_ref() {
            render_area.push_arg(&mut out);
        }

        if let Some(render_color) = self.render_color.as_ref() {
            render_color.push_arg(&mut out);
        }

        if let Some(page_color) = self.page_color.as_ref() {
            page_color.push_arg(&mut out);
        }

        if let Some(password) = self.password.as_ref() {
            password.push_arg(&mut out);
        }

        out
    }
}

/// Color to use as the background of pages
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PageColor {
    #[default]
    White,
    /// Only supported on PNG/TIFF [OutputType]s
    Transparent,
}

impl PageColor {
    pub fn push_arg(&self, args: &mut Vec<String>) {
        match self {
            Self::White => {}
            Self::Transparent => args.push("-transp".to_string()),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Antialias {
    /// Use the default antialiasing for the target device.
    #[default]
    Default,
    /// Antialiasing is disabled.
    None,
    /// Perform single-color antialiasing using shades of gray.
    Gray,
    /// Perform  antialiasing  by  taking advantage of the order of subpixel elements on de‐
    /// vices such as LCD.
    Subpixel,
    /// Hint that the backend should perform some antialiasing but prefer speed  over  quality.
    Fast,
    /// The backend should balance quality against performance.
    Good,
    /// Hint  that  the  backend  should render at the highest quality, sacrificing speed if necessary.
    Best,
}

impl Antialias {
    pub fn push_arg(&self, args: &mut Vec<String>) {
        args.push("-anti".to_string());

        match self {
            Self::Default => args.push("default".to_string()),
            Self::None => args.push("none".to_string()),
            Self::Gray => args.push("gray".to_string()),
            Self::Subpixel => args.push("subpixel".to_string()),
            Self::Fast => args.push("fast".to_string()),
            Self::Good => args.push("good".to_string()),
            Self::Best => args.push("best".to_string()),
        };
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RenderColor {
    #[default]
    Color,
    Monochrome,
    Grayscale,
}

impl RenderColor {
    pub fn push_arg(&self, args: &mut Vec<String>) {
        match self {
            Self::Color => {}
            Self::Monochrome => args.push("-mono".to_string()),
            Self::Grayscale => args.push("-gray".to_string()),
        };
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RenderArea {
    #[default]
    MediaBox,
    CropBox,
}

impl RenderArea {
    pub fn push_arg(&self, args: &mut Vec<String>) {
        match self {
            Self::MediaBox => {}
            Self::CropBox => args.push("-cropbox".to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Crop {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl Crop {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn uniform(x: u32, y: u32, size: u32) -> Self {
        Self::new(x, y, size, size)
    }

    pub fn push_arg(&self, args: &mut Vec<String>) {
        args.push("-x".to_string());
        args.push(self.x.to_string());

        args.push("-y".to_string());
        args.push(self.y.to_string());

        args.push("-W".to_string());
        args.push(self.width.to_string());

        args.push("-H".to_string());
        args.push(self.height.to_string());
    }
}

/// Scales the output image to fit inside the provided size
#[derive(Debug, Clone, Copy)]
pub struct ScaleTo {
    /// The X bounds to scale to fit within
    x: i32,
    /// The Y bounds to scale to fit within
    y: i32,
}

impl Default for ScaleTo {
    fn default() -> Self {
        Self::new(Self::MAINTAIN_ASPECT_RATIO, Self::MAINTAIN_ASPECT_RATIO)
    }
}

impl ScaleTo {
    pub const MAINTAIN_ASPECT_RATIO: i32 = -1;

    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn x(x: i32) -> Self {
        Self {
            x,
            y: Self::MAINTAIN_ASPECT_RATIO,
        }
    }

    pub fn y(y: i32) -> Self {
        Self {
            x: Self::MAINTAIN_ASPECT_RATIO,
            y,
        }
    }

    pub fn uniform(scale: i32) -> Self {
        Self::new(scale, scale)
    }

    pub fn push_arg(&self, args: &mut Vec<String>) {
        args.push("-scale-to-x".to_string());
        args.push(self.x.to_string());

        args.push("-scale-to-y".to_string());
        args.push(self.y.to_string());
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Resolution {
    /// X resolution in pixels per inch
    x: u32,
    /// Y resolution in pixels per inch
    y: u32,
}

impl Default for Resolution {
    fn default() -> Self {
        Self::uniform(150)
    }
}

impl Resolution {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    pub fn x(x: u32) -> Self {
        Self { x, y: 150 }
    }

    pub fn y(y: u32) -> Self {
        Self { x: 150, y }
    }

    pub fn uniform(size: u32) -> Self {
        Self::new(size, size)
    }

    pub fn push_arg(&self, args: &mut Vec<String>) {
        args.push("-rx".to_string());
        args.push(self.x.to_string());

        args.push("-ry".to_string());
        args.push(self.y.to_string());
    }
}

/// Output formats for pdftocairo, the program
/// supports other formats but we only use these
/// types
#[derive(Debug, Default, Clone, Copy)]
pub enum OutputFormat {
    /// Portable Network Graphics (PNG)
    Png,
    /// JPEG Interchange Format (JPEG)
    #[default]
    Jpeg,
    /// Tagged Image File Format (TIFF)
    Tiff,
}

impl OutputFormat {
    pub fn push_arg(&self, args: &mut Vec<String>) {
        args.push(match self {
            OutputFormat::Png => "-png".to_string(),
            OutputFormat::Jpeg => "-jpeg".to_string(),
            OutputFormat::Tiff => "-tiff".to_string(),
        });
    }

    pub fn image_format(&self) -> ImageFormat {
        match self {
            OutputFormat::Png => ImageFormat::Png,
            OutputFormat::Jpeg => ImageFormat::Jpeg,
            OutputFormat::Tiff => ImageFormat::Tiff,
        }
    }
}

#[derive(Debug, Error)]
pub enum PdfRenderError {
    #[error("failed to spawn pdftocairo: {0}")]
    SpawnProcess(std::io::Error),

    #[error("failed to write pdf bytes: {0}")]
    WritePdf(std::io::Error),

    #[error("failed to get output: {0}")]
    WaitOutput(std::io::Error),

    #[error("failed to get pdftocairo exit code: {0}")]
    PdfRenderFailure(String),

    #[error("pdftocairo reported permission error: {0}")]
    PermissionError(String),

    #[error(transparent)]
    Image(ImageError),

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

/// Renders all the pages in the provided PDF in parallel.
///
/// If you only want a specific page use [render_single_page]
///
/// ## Arguments
/// * data - The raw PDF file bytes
/// * info - The PDF info to use for the page count and encryption state
/// * format - The output format to render as
/// * args - Optional args to pdftocairo
pub async fn render_all_pages(
    data: &[u8],
    info: &PdfInfo,
    format: OutputFormat,
    args: &RenderArgs,
) -> Result<Vec<DynamicImage>, PdfRenderError> {
    // Check encryption
    if info.encrypted().unwrap_or_default() && args.password.is_none() {
        return Err(PdfRenderError::PdfEncrypted);
    }

    // Get the page count
    let page_count = info
        .pages()
        .ok_or(PdfRenderError::PageCountUnknown)?
        .map_err(|_| PdfRenderError::PageCountUnknown)?;

    // Render all the pages individually
    (1..=page_count)
        .map(|page| render_page(data, format, page, args))
        .collect::<FuturesOrdered<_>>()
        .try_collect()
        .await
}

/// Renders all the provided pages in parallel
///
/// If you only want a specific page use [render_single_page]
///
/// ## Arguments
/// * data - The raw PDF file bytes
/// * info - The PDF info to use for the page count and encryption state
/// * format - The output format to render as
/// * pages - The list of page numbers to render
/// * args - Optional args to pdftocairo
pub async fn render_pages(
    data: &[u8],
    info: &PdfInfo,
    format: OutputFormat,
    pages: Vec<u32>,
    args: &RenderArgs,
) -> Result<Vec<DynamicImage>, PdfRenderError> {
    // Check encryption
    if info.encrypted().unwrap_or_default() && args.password.is_none() {
        return Err(PdfRenderError::PdfEncrypted);
    }

    // Get the page count
    let page_count = info
        .pages()
        .ok_or(PdfRenderError::PageCountUnknown)?
        .map_err(|_| PdfRenderError::PageCountUnknown)?;

    // Validate requested pages
    for page in &pages {
        if *page > page_count {
            return Err(PdfRenderError::PageOutOfBounds(*page, page_count));
        }
    }

    // Render all the pages individually
    pages
        .into_iter()
        .map(|page| render_page(data, format, page, args))
        .collect::<FuturesOrdered<_>>()
        .try_collect()
        .await
}

/// Renders a single page from a PDF file
///
/// ## Arguments
/// * data - The raw PDF file bytes
/// * format - The output format to render as
/// * page - The page to render
/// * args - Optional args to pdftocairo
pub async fn render_single_page(
    data: &[u8],
    info: &PdfInfo,
    format: OutputFormat,
    page: u32,
    args: &RenderArgs,
) -> Result<DynamicImage, PdfRenderError> {
    // Check encryption
    if info.encrypted().unwrap_or_default() && args.password.is_none() {
        return Err(PdfRenderError::PdfEncrypted);
    }

    // Get the page count
    let page_count = info
        .pages()
        .ok_or(PdfRenderError::PageCountUnknown)?
        .map_err(|_| PdfRenderError::PageCountUnknown)?;

    // Validate chosen page
    if page > page_count {
        return Err(PdfRenderError::PageOutOfBounds(page, page_count));
    }

    render_page(data, format, page, args).await
}

/// Renders the provided page from a pdf file using `pdftocairo`
pub(crate) async fn render_page(
    data: &[u8],
    format: OutputFormat,
    page: u32,
    args: &RenderArgs,
) -> Result<DynamicImage, PdfRenderError> {
    let mut cli_args = args.build_args();
    format.push_arg(&mut cli_args);

    let mut child = Command::new("pdftocairo")
        // Take input from stdin and provide to stdout
        .args(["-", "-"])
        // Specify first and last pages
        .args([
            "-singlefile",
            "-f",
            &page.to_string(),
            "-l",
            &page.to_string(),
        ])
        // Add optional args and output format
        .args(cli_args)
        // Pipe input and output for use
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(PdfRenderError::SpawnProcess)?;

    child
        .stdin
        .as_mut()
        // Should always have stdin when using .stdin(Stdio::piped())
        .expect("progress missing stdin after being piped")
        .write_all(data)
        .await
        .map_err(PdfRenderError::WritePdf)?;

    let output = child
        .wait_with_output()
        .await
        .map_err(PdfRenderError::WaitOutput)?;

    // Handle info failure
    if !output.status.success() {
        let value = String::from_utf8_lossy(&output.stderr);

        if value.contains("May not be a PDF file") {
            return Err(PdfRenderError::NotPdfFile);
        }

        if value.contains("Incorrect password") {
            return Err(if args.password.is_none() {
                PdfRenderError::PdfEncrypted
            } else {
                PdfRenderError::IncorrectPassword
            });
        }

        let code = output.status.code();

        match code {
            Some(3) => return Err(PdfRenderError::PermissionError(value.to_string())),
            _ => return Err(PdfRenderError::PdfRenderFailure(value.to_string())),
        }
    }

    let image = image::load_from_memory_with_format(&output.stdout, format.image_format())
        .map_err(PdfRenderError::Image)?;

    Ok(image)
}

#[cfg(test)]
mod test {
    use super::{
        render_all_pages, render_page, render_pages, render_single_page, PdfRenderError, RenderArgs,
    };
    use crate::{
        info::{pdf_info, PdfInfoArgs},
        shared::{Password, Secret},
    };
    use tokio::fs::read;

    /// Tests invalid files are handled
    #[tokio::test]
    async fn test_invalid_file() {
        let value = &[b'A'];
        let args = RenderArgs::default();
        let err = render_page(value, crate::image::OutputFormat::Jpeg, 1, &args)
            .await
            .unwrap_err();
        assert!(matches!(err, PdfRenderError::NotPdfFile));
    }

    /// Tests rendering all pages
    #[tokio::test]
    async fn test_all_pages() {
        let data = read("./tests/samples/test-pdf-2-pages.pdf").await.unwrap();
        let info = pdf_info(&data, &PdfInfoArgs::default()).await.unwrap();
        let args = RenderArgs::default();
        let output = render_all_pages(&data, &info, crate::image::OutputFormat::Jpeg, &args)
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

        let _output = render_single_page(&data, &info, crate::image::OutputFormat::Jpeg, 1, &args)
            .await
            .unwrap();
    }

    /// Tests rendering a specific set of pages
    #[tokio::test]
    async fn test_specific_pages() {
        let data = read("./tests/samples/test-pdf-2-pages.pdf").await.unwrap();

        let info = pdf_info(&data, &PdfInfoArgs::default()).await.unwrap();
        let args = RenderArgs::default();

        let output = render_pages(
            &data,
            &info,
            crate::image::OutputFormat::Jpeg,
            vec![1, 2],
            &args,
        )
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

        let err = render_single_page(&data, &info, crate::image::OutputFormat::Jpeg, 99, &args)
            .await
            .unwrap_err();
        assert!(matches!(err, PdfRenderError::PageOutOfBounds(99, 2)));

        let err = render_pages(
            &data,
            &info,
            crate::image::OutputFormat::Jpeg,
            vec![99],
            &args,
        )
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

        let info_args = PdfInfoArgs {
            password: Some(Password::User(Secret("password".to_string()))),
        };

        let info = pdf_info(&data, &info_args).await.unwrap();
        let args = RenderArgs::default();

        let err = render_single_page(&data, &info, crate::image::OutputFormat::Jpeg, 99, &args)
            .await
            .unwrap_err();
        assert!(matches!(err, PdfRenderError::PdfEncrypted));

        let err = render_pages(
            &data,
            &info,
            crate::image::OutputFormat::Jpeg,
            vec![99],
            &args,
        )
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

        let info_args = PdfInfoArgs {
            password: Some(Password::User(Secret("password".to_string()))),
        };

        let info = pdf_info(&data, &info_args).await.unwrap();
        let args = RenderArgs {
            password: Some(Password::User(Secret("password".to_string()))),
            ..Default::default()
        };

        let _output = render_single_page(&data, &info, crate::image::OutputFormat::Jpeg, 2, &args)
            .await
            .unwrap();

        let output = render_all_pages(&data, &info, crate::image::OutputFormat::Jpeg, &args)
            .await
            .unwrap();

        assert_eq!(output.len(), 2);

        let output = render_pages(
            &data,
            &info,
            crate::image::OutputFormat::Jpeg,
            vec![1, 2],
            &args,
        )
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

        let info_args = PdfInfoArgs {
            password: Some(Password::User(Secret("password".to_string()))),
        };

        let info = pdf_info(&data, &info_args).await.unwrap();
        let args = RenderArgs {
            password: Some(Password::User(Secret("incorrect".to_string()))),
            ..Default::default()
        };
        let err = render_single_page(&data, &info, crate::image::OutputFormat::Jpeg, 1, &args)
            .await
            .unwrap_err();
        assert!(matches!(err, PdfRenderError::IncorrectPassword));

        let err = render_pages(
            &data,
            &info,
            crate::image::OutputFormat::Jpeg,
            vec![1],
            &args,
        )
        .await
        .unwrap_err();

        assert!(matches!(err, PdfRenderError::IncorrectPassword));
    }
}
