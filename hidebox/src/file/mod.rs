use crate::util;
use anyhow::{anyhow, Result};
use std::{
    fs::{self, File},
    io::{Read, SeekFrom, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

const CHUNK_SIZE: usize = 4096;
const CHUNK_LEN_SIZE: usize = 8;
const HASH_TEXT_SIZE: usize = 32;
const MIN_CHUNK_LEN: usize = CHUNK_LEN_SIZE + HASH_TEXT_SIZE;
const MAGIC_NUM: &str = "HIDEBOX";
const MAX_FILE_SIZE: u64 = 4 * 1024 * 1024 * 1024; // 4G

pub static CANCEL_ENCODE: AtomicBool = AtomicBool::new(false);
pub static CANCEL_DECODE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Default, Debug)]
struct ChunkSpec {
    data: Vec<u8>,
}

fn hex_str(num: u64) -> Option<String> {
    if num >= MAX_FILE_SIZE {
        None
    } else {
        Some(format!("{:8x}", num))
    }
}

// LAYOUT: data_len(8 bytes) + encrypt_text + hash_text(32 bytes);
//  data_len = encrypt_text.len + hash_text.len;
//  Note: buffer.len >= 4096
fn make_chunk(password: &str, buffer: &[u8]) -> Result<Vec<u8>> {
    let encrypt_text = util::crypto::encrypt(password, buffer)?;
    let hash_text = util::crypto::hash(&encrypt_text);
    let text_len = encrypt_text.len() + hash_text.len();
    let hex_text_len = match hex_str(text_len as u64) {
        Some(v) => v,
        None => return Err(anyhow!("buffer is too large")),
    };

    let mut chunk = Vec::with_capacity(hex_text_len.len() + text_len);
    chunk.extend_from_slice(hex_text_len.as_bytes());
    chunk.extend_from_slice(encrypt_text.as_bytes());
    chunk.extend_from_slice(hash_text.as_bytes());

    Ok(chunk)
}

fn get_chunk_from_buffer(buffer: &[u8]) -> Result<Vec<u8>> {
    if buffer.len() <= MIN_CHUNK_LEN {
        return Err(anyhow!("buffer is too small, less than {MIN_CHUNK_LEN}"));
    }

    let text_len = String::from_utf8_lossy(&buffer[..CHUNK_LEN_SIZE])
        .trim()
        .to_string();

    if text_len.is_empty() {
        return Err(anyhow!("chunk length is empty"));
    }

    let text_len = usize::from_str_radix(&text_len, 16)?;

    if text_len > buffer.len() - CHUNK_LEN_SIZE {
        return Err(anyhow!("chunk length is larger than buffer length"));
    }

    let chunk = &buffer[..CHUNK_LEN_SIZE + text_len].to_vec();
    Ok(chunk.clone())
}

fn parse_chunk(password: &str, buffer: &[u8]) -> Result<ChunkSpec> {
    let encrypt_text = &buffer[CHUNK_LEN_SIZE..buffer.len() - HASH_TEXT_SIZE];
    let encrypt_text = String::from_utf8_lossy(encrypt_text);

    let hash_text = &buffer[buffer.len() - HASH_TEXT_SIZE..];
    let hash_text = String::from_utf8_lossy(hash_text);

    if hash_text != util::crypto::hash(&encrypt_text) {
        return Err(anyhow!("invalid chunk checksum"));
    }

    let data = util::crypto::decrypt(password, &encrypt_text)?;
    if data.len() > CHUNK_SIZE {
        return Err(anyhow!("invalid chunk, chunk size is too larger"));
    }

    Ok(ChunkSpec { data })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWORD: &str = "123456";

    #[test]
    fn test_file_hex_str() {
        let s = super::hex_str(MAX_FILE_SIZE - 1);
        assert!(s.is_some());
    }

    #[test]
    fn test_file_make_chunk() -> Result<()> {
        let buffer = util::str::random_string(CHUNK_SIZE);
        let chunk = make_chunk(PASSWORD, buffer.as_bytes())?;
        let chunk_str = String::from_utf8_lossy(chunk.as_slice());
        assert!(chunk_str.len() > CHUNK_SIZE);
        Ok(())
    }

    #[test]
    fn test_file_get_chunk_from_buffer() -> Result<()> {
        let buffer = util::str::random_string(CHUNK_SIZE);
        let chunk = make_chunk(PASSWORD, buffer.as_bytes())?;
        let chunk_2 = get_chunk_from_buffer(chunk.as_slice())?;
        assert_eq!(chunk, chunk_2);

        Ok(())
    }

    #[test]
    fn test_file_parse_chunk() -> Result<()> {
        let buffer = util::str::random_string(CHUNK_SIZE);
        let chunk = make_chunk(PASSWORD, buffer.as_bytes())?;
        let chunk_2 = get_chunk_from_buffer(chunk.as_slice())?;
        assert_eq!(chunk, chunk_2);

        let cs = parse_chunk(PASSWORD, chunk_2.as_slice())?;
        assert_eq!(cs.data, buffer.as_bytes());

        Ok(())
    }

    // #[tokio::test]
    // async fn test_feerate() -> Result<()> {
    //     let (low, middle, high) = super::feerate().await?;
    //     assert!(low > 0);
    //     assert!(middle >= low);
    //     assert!(high >= middle);

    //     // println!("{}, {}, {}", low, middle, high);
    //     Ok(())
    // }
}
