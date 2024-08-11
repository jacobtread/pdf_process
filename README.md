# Pdf Process

Library for processing PDF files in Rust, wraps the CLI utilities provided by [Poppler](https://poppler.freedesktop.org/) specifically `pdftotext` (Text extraction), `pdftocairo` (Image rendering), `pdfinfo` (Extracting basic details)

Provides functionality for:
- Extracting PDF text contents
- Rendering PDF files to images (PNG/JPEG/TIFF)
- Basic PDF Details (Encryption, Page Count, Subject, Title, Creator, Author, etc..)

## Prerequisites

> Library developed against a Linux host. Windows is not supported

Requires [Plopper](https://poppler.freedesktop.org/) be installed on your system and the utilities on your `PATH`. Lots 
of distributions will come with this pre-installed. You can check if its installed by using `pdfinfo -v` which should 
produce an output similar to:

```sh
pdfinfo version 24.02.0
Copyright 2005-2024 The Poppler Developers - http://poppler.freedesktop.org
Copyright 1996-2011, 2022 Glyph & Cog, LLC,
```

Otherwise you can install it with one of the commands below:

**Fedora**:

```sh
sudo dnf install poppler-utils
```

Adjust the command above for your specific Linux distribution

## Installation

Install with cargo:

```sh
cargo add pdf_process
```

Or add the following to the `[dependencies]` section of your `Cargo.toml`:

```toml
pdf_process = "0.1.0"
```

## Tested

**Tested against**:

pdftotext version 24.02.0
pdftocairo version 24.02.0
pdfinfo version 24.02.0