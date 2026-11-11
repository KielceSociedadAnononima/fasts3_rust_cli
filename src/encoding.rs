use crate::error::{FastS3Error, Result};

/// Percent-encodes a single URL path segment according to RFC 3986 unreserved characters.
pub(crate) fn encode_path_segment(segment: &str) -> String {
    let mut encoded = String::with_capacity(segment.len());
    for &byte in segment.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push( hex_upper(byte >> 4) );
            encoded.push( hex_upper(byte & 0x0F) );
        }
    }
    encoded
}

/// Validates and percent-encodes a multi-segment S3 object key while preserving `/` separators.
pub(crate) fn encode_object_key(key: &str) -> Result<String> {
    let normalized = key.replace('\\', "/");
    let segments: Vec<String> = normalized
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|seg| {
            if seg == "." || seg == ".." {
                Err(FastS3Error::InvalidArgument(format!(
                    "Object key cannot contain relative traversal segment '{seg}'"
                )))
            } else {
                Ok(encode_path_segment(seg))
            }
        })
        .collect::<Result<Vec<_>>>()?;

    if segments.is_empty() {
        return Err(FastS3Error::InvalidArgument(
            "Object key cannot be empty".to_string(),
        ));
    }

    Ok(segments.join("/"))
}

/// Extracts `filename="..."` from a `Content-Disposition` HTTP header if present.
pub(crate) fn parse_content_disposition_filename(header_value: &str) -> Option<String> {
    for part in header_value.split(';') {
        let trimmed = part.trim();
        if let Some(rest) = trimmed.strip_prefix("filename=") {
            let unquoted = rest.trim_matches('"').trim();
            if !unquoted.is_empty() {
                return Some(unquoted.to_string());
            }
        }
    }
    None
}

#[inline]
const fn hex_upper(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        _ => (b'A' + (nibble - 10)) as char,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_path_segment() {
        assert_eq!(encode_path_segment("my-bucket_01"), "my-bucket_01");
        assert_eq!(encode_path_segment("hello world#1?"), "hello%20world%231%3F");
    }

    #[test]
    fn test_encode_object_key() {
        assert_eq!(
            encode_object_key("/avatars/2026/carol photo.png").unwrap(),
            "avatars/2026/carol%20photo.png"
        );
        assert!(encode_object_key("").is_err());
        assert!(encode_object_key("///").is_err());
        assert!(encode_object_key("a/../b").is_err());
    }

    #[test]
    fn test_parse_content_disposition() {
        assert_eq!(
            parse_content_disposition_filename("inline; filename=\"report.pdf\""),
            Some("report.pdf".to_string())
        );
        assert_eq!(parse_content_disposition_filename("inline"), None);
    }
}
