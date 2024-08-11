# Pdf Process

Library for processing PDF files in Rust, wraps the CLI utilities provided by [Plopper](https://poppler.freedesktop.org/) specifically `pdftotext` (Text extraction), `pdftocairo` (Image rendering), `pdfinfo` (Extracting basic details)

Provides functionality for:
- Extracting PDF text contents
- Rendering PDF files to images (PNG/JPEG/TIFF)
- Basic PDF Details (Encryption, Page Count, Subject, Title, Creator, Author, etc..)

## Tested

**Tested against**:

pdftotext version 24.02.0
pdftocairo version 24.02.0
pdfinfo version 24.02.0