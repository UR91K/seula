use std::io::Write;

use seula::error::FileError;
use seula::utils::decompress_gzip_file;

/// The first bytes of `4-DUNE.als`, a pre-8.2 Live set. It used to fail with "invalid
/// gzip header", which read like a corrupt file.
#[test]
fn test_pre_8_2_set_is_unsupported_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.als");
    std::fs::write(&path, [0xAB, 0x1E, 0x56, 0x78, 0x03, 0x7C, 0x00, 0x00, 0x00, 0x00]).unwrap();

    match decompress_gzip_file(&path) {
        Err(FileError::UnsupportedVersion { path: p }) => assert_eq!(p, path),
        other => panic!("expected UnsupportedVersion, got {other:?}"),
    }
}

/// Anything else that is not gzip keeps the ordinary error.
#[test]
fn test_other_non_gzip_still_a_decompression_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("junk.als");
    std::fs::write(&path, b"not gzip at all").unwrap();

    assert!(matches!(
        decompress_gzip_file(&path),
        Err(FileError::GzipDecompressionError { .. })
    ));
}

#[test]
fn test_gzip_still_decompresses() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ok.als");
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(b"<Ableton/>").unwrap();
    std::fs::write(&path, enc.finish().unwrap()).unwrap();

    assert_eq!(decompress_gzip_file(&path).unwrap(), b"<Ableton/>");
}
