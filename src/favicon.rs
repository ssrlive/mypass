use crate::error::Result;
use regex::Regex;
use reqwest::blocking::Client;
use std::io::Cursor;
use std::path::Path;
use std::time::Duration;
use url::Url;

const DEFAULT_FAVICON_PATH: &str = "/favicon.ico";

/// Decodes raw image bytes into an image object, including SVG data.
pub fn image_from_bytes(bytes: &[u8]) -> Result<image::DynamicImage> {
    if is_svg_bytes(bytes) {
        let png_bytes = svg_to_png_bytes(bytes)?;
        return Ok(image::load_from_memory(&png_bytes)?);
    }
    Ok(image::load_from_memory(bytes)?)
}

/// The downloaded favicon bytes and the metadata needed to save or inspect them.
pub struct DownloadedFavicon {
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
    pub source_url: Url,
}

impl DownloadedFavicon {
    /// Decodes the downloaded data into an image object.
    pub fn to_image(&self) -> Result<image::DynamicImage> {
        image_from_bytes(&self.bytes)
    }

    /// Decodes the downloaded image and returns it encoded as PNG bytes.
    pub fn to_png_bytes(&self) -> Result<Vec<u8>> {
        let image = self.to_image()?;
        let mut png_bytes = Cursor::new(Vec::new());
        image.write_to(&mut png_bytes, image::ImageFormat::Png)?;
        Ok(png_bytes.into_inner())
    }

    /// Saves the original bytes without decoding or converting the image format.
    pub fn save_to(&self, path: impl AsRef<Path>) -> Result<()> {
        std::fs::write(path, &self.bytes)?;
        Ok(())
    }

    /// Returns a useful extension based on the server MIME type or source URL.
    pub fn suggested_extension(&self) -> Option<&'static str> {
        self.content_type
            .as_deref()
            .and_then(extension_for_content_type)
            .or_else(|| extension_for_path(self.source_url.path()))
    }
}

fn is_svg_bytes(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok_and(|value| {
        let value = value.trim_start();
        value.starts_with("<svg") || (value.starts_with("<?xml") && value.contains("<svg"))
    })
}

fn svg_to_png_bytes(bytes: &[u8]) -> Result<Vec<u8>> {
    let tree =
        resvg::usvg::Tree::from_data(bytes, &resvg::usvg::Options::default()).map_err(|error| format!("SVG conversion error: {error}"))?;
    let size = tree.size().to_int_size();
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(size.width(), size.height()).ok_or_else(|| "SVG has an invalid or empty size".to_string())?;
    resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|error| format!("PNG encoding error: {error}").into())
}

/// Downloads favicons declared by a website, with `/favicon.ico` as a fallback.
pub struct FaviconDownloader {
    client: Client,
}

impl FaviconDownloader {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .user_agent("mypass favicon downloader")
                .timeout(Duration::from_secs(10))
                .build()?,
        })
    }

    /// Finds and downloads the first usable favicon for the supplied website URL.
    pub fn download(&self, website_url: &str) -> Result<Option<DownloadedFavicon>> {
        let website_url = Url::parse(website_url)?;
        if !matches!(website_url.scheme(), "http" | "https") {
            return Err(format!("unsupported website URL scheme: {}", website_url.scheme()).into());
        }

        let page = self.client.get(website_url.clone()).send();
        let (page_url, html) = match page {
            Ok(response) => {
                let page_url = response.url().clone();
                if response.status().is_success() {
                    (page_url, response.text()?)
                } else {
                    (page_url, String::new())
                }
            }
            Err(_) => (website_url.clone(), String::new()),
        };
        let mut candidates = favicon_links(&html)
            .into_iter()
            .filter_map(|href| page_url.join(&href).ok())
            .collect::<Vec<_>>();
        if let Ok(fallback) = page_url.join(DEFAULT_FAVICON_PATH) {
            candidates.push(fallback);
        }

        for candidate in candidates {
            match self.download_candidate(candidate) {
                Ok(Some(favicon)) => return Ok(Some(favicon)),
                Ok(None) | Err(_) => continue,
            }
        }
        Ok(None)
    }

    fn download_candidate(&self, url: Url) -> Result<Option<DownloadedFavicon>> {
        let response = self.client.get(url.clone()).send()?;
        if !response.status().is_success() {
            return Ok(None);
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let bytes = response.bytes()?.to_vec();
        if bytes.is_empty() {
            return Ok(None);
        }
        Ok(Some(DownloadedFavicon {
            bytes,
            content_type,
            source_url: url,
        }))
    }
}

fn favicon_links(html: &str) -> Vec<String> {
    let tag_regex = Regex::new(r"(?is)<link\b[^>]*>").expect("favicon tag regex is valid");
    let attribute_regex = Regex::new(r#"(?is)([a-z_:][-a-z0-9_:]*)\s*=\s*[\"']([^\"']*)[\"']"#).expect("favicon attribute regex is valid");
    tag_regex
        .find_iter(html)
        .filter_map(|tag| {
            let mut rel = None;
            let mut href = None;
            for captures in attribute_regex.captures_iter(tag.as_str()) {
                match captures[1].to_ascii_lowercase().as_str() {
                    "rel" => rel = Some(captures[2].to_owned()),
                    "href" => href = Some(captures[2].to_owned()),
                    _ => {}
                }
            }
            let is_icon = rel.is_some_and(|value| {
                value.split_ascii_whitespace().any(|token| {
                    matches!(
                        token.to_ascii_lowercase().as_str(),
                        "icon" | "shortcut" | "apple-touch-icon" | "mask-icon"
                    )
                })
            });
            is_icon.then_some(href?).filter(|href| !href.trim().is_empty())
        })
        .collect()
}

fn extension_for_content_type(content_type: &str) -> Option<&'static str> {
    match content_type.split(';').next()?.trim().to_ascii_lowercase().as_str() {
        "image/x-icon" | "image/vnd.microsoft.icon" => Some("ico"),
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/gif" => Some("gif"),
        "image/svg+xml" => Some("svg"),
        "image/webp" => Some("webp"),
        "image/avif" => Some("avif"),
        _ => None,
    }
}

fn extension_for_path(path: &str) -> Option<&'static str> {
    match Path::new(path).extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "ico" => Some("ico"),
        "png" => Some("png"),
        "jpg" | "jpeg" => Some("jpg"),
        "gif" => Some("gif"),
        "svg" => Some("svg"),
        "webp" => Some("webp"),
        "avif" => Some("avif"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{DownloadedFavicon, favicon_links, image_from_bytes};
    use url::Url;

    #[test]
    fn finds_declared_favicon_links() {
        let html = r#"
            <link rel="stylesheet" href="style.css">
            <link rel="icon" type="image/svg+xml" href="/icon.svg">
            <link rel="apple-touch-icon" href="icons/touch.png">
        "#;
        assert_eq!(favicon_links(html), vec!["/icon.svg", "icons/touch.png"]);
    }

    #[test]
    fn converts_downloaded_image_to_png() {
        let image = image::DynamicImage::new_rgba8(2, 2);
        let mut source = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut source, image::ImageFormat::Png)
            .expect("failed to encode test image");
        let favicon = DownloadedFavicon {
            bytes: source.into_inner(),
            content_type: Some("image/png".to_string()),
            source_url: Url::parse("https://example.com/icon.png").expect("valid test URL"),
        };
        let decoded = favicon.to_image().expect("failed to decode image");
        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);
        let png = favicon.to_png_bytes().expect("failed to convert favicon to PNG");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn converts_downloaded_svg_to_png() {
        let raw_svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2"><rect width="2" height="2" fill="red"/></svg>"#;
        let decoded = image_from_bytes(raw_svg).expect("failed to decode SVG");
        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);

        let favicon = DownloadedFavicon {
            bytes: raw_svg.to_vec(),
            content_type: Some("image/svg+xml".to_string()),
            source_url: Url::parse("https://example.com/icon.svg").expect("valid test URL"),
        };
        let decoded = favicon.to_image().expect("failed to decode SVG");
        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);
        let png = favicon.to_png_bytes().expect("failed to convert SVG to PNG");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn test_download_favicon() {
        let downloader = super::FaviconDownloader::new().expect("failed to create downloader");
        let result = downloader
            .download("https://rustcc.cn/article?id=27f3d3bd-9446-434e-a64b-b4c3ade74c2f")
            .expect("download failed");
        assert!(result.is_some(), "no favicon found");
        let favicon = result.unwrap();
        assert!(!favicon.bytes.is_empty(), "favicon bytes are empty");
        assert_eq!(favicon.source_url.host_str(), Some("rustcc.cn"));
    }
}
