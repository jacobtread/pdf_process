pub mod image;
pub mod info;
pub mod shared;
pub mod text;

pub use image::{
    render_all_pages, render_pages, render_single_page, Antialias, Crop, OutputFormat, PageColor,
    PdfRenderError, RenderArea, RenderArgs, RenderColor, Resolution, ScaleTo,
};
pub use info::{pdf_info, PdfInfo, PdfInfoArgs, PdfInfoError};
pub use shared::{Password, Secret};
pub use text::{
    text_all_pages, text_all_pages_split, text_pages, text_single_page, PdfTextArgs, PdfTextError,
};
