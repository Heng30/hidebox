use super::*;
use crate::{util, util::translator::tr};
use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

static CANCEL_ENCODE: AtomicBool = AtomicBool::new(false);

pub fn cancel() {
    CANCEL_ENCODE.store(true, Ordering::SeqCst);
}

fn hex_str(num: u64) -> Option<String> {
    if num > MAX_FILE_SIZE {
        None
    } else {
        Some(format!("{:8x}", num))
    }
}

// LAYOUT: data_len(8 bytes) + encrypt_text + hash_text(32 bytes);
//  data_len = encrypt_text.len + hash_text.len;
//  Note: buffer.len >= 4096
pub fn make_chunk(password: &str, buffer: &[u8]) -> Result<Vec<u8>> {
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

// save append_file in to src_file and append append_file info to the end;
pub async fn encode(
    src_file_spec: FileSpec,
    append_file_spec: FileSpec,
    output_file: &Path,
    password: &str,
    progress_callback: ProgressCb,
    mut progress_callback_arg: ProgressCbArg,
) -> Result<String> {
    CANCEL_ENCODE.store(false, Ordering::SeqCst);

    let mut src_file = File::open(&src_file_spec.path).await?;
    let mut append_file = File::open(&append_file_spec.path).await?;
    let mut output_file = File::create(&output_file).await?;

    let total = src_file_spec.size + append_file_spec.size;
    log::debug!(
        "src-size:{} append-size:{} total:{}",
        src_file_spec.size,
        append_file_spec.size,
        total
    );

    let mut current = 0;
    let mut total_chunks = 0;
    let mut buf = [0; CHUNK_SIZE];

    // write src file
    loop {
        let len = src_file.read(&mut buf).await?;
        if len == 0 {
            break;
        }

        output_file.write_all(&buf[0..len]).await?;
        current += len;
        total_chunks += 1;

        if CANCEL_ENCODE.load(Ordering::SeqCst) {
            return Ok(tr("取消成功"));
        }

        if total_chunks % 10 == 0 {
            let progress = ((current as f64 / total as f64) * 100.) as u32;
            progress_callback_arg.progress = progress;
            progress_callback(progress_callback_arg.clone());
            // log::debug!("current={} total={} progress={progress}", current, total);
        }

        // the last chunk of the file is written
        if len < buf.len() {
            break;
        }
    }

    output_file.write_all(MAGIC_NUM.as_bytes()).await?;

    let mut append_encrypt_total_size = 0;

    // write append file
    loop {
        let len = append_file.read(&mut buf).await?;
        if len == 0 {
            break;
        }

        let encrypt_buf = make_chunk(password, &buf[0..len])?;
        output_file.write_all(&encrypt_buf).await?;

        current += len;
        total_chunks += 1;
        append_encrypt_total_size += encrypt_buf.len();

        if append_encrypt_total_size as u64 > MAX_FILE_SIZE {
            return Err(anyhow!("append file is too big"));
        }

        if CANCEL_ENCODE.load(Ordering::SeqCst) {
            return Ok(tr("取消成功"));
        }

        if total_chunks % 10 == 0 {
            let progress = ((current as f64 / total as f64) * 100.) as u32;
            progress_callback_arg.progress = progress;
            progress_callback(progress_callback_arg.clone());
            // log::debug!("current={} total={} progress={progress}", current, total);
        }

        if len < buf.len() {
            break;
        }
    }

    let hide_spec = HideSpec {
        append_name: append_file_spec.name,
        append_size: append_encrypt_total_size as u64,
        src_size: src_file_spec.size,
    };
    let hide_spec_data = serde_json::to_string(&hide_spec)?;
    let hide_spec_data = util::crypto::encrypt(password, hide_spec_data.as_bytes())?;
    let hide_spec_data_len = hex_str(hide_spec_data.len() as u64).unwrap();

    output_file.write_all(hide_spec_data.as_bytes()).await?;
    output_file.write_all(hide_spec_data_len.as_bytes()).await?;
    output_file.write_all(MAGIC_NUM.as_bytes()).await?;

    progress_callback_arg.progress = 100;
    progress_callback(progress_callback_arg);

    Ok(tr("写入成功"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

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

    fn pcb(arg: ProgressCbArg) {
        println!("{}", arg.progress);
    }

    #[tokio::test]
    async fn test_file_encode_less_than_4k() -> Result<()> {
        let cur_dir = env::current_dir()?;

        let src_file_path = cur_dir
            .clone()
            .join("../testdata/src.dat")
            .to_str()
            .unwrap()
            .to_string();

        let append_file_path = cur_dir
            .clone()
            .join("../testdata/append-less-than-4k.dat")
            .to_str()
            .unwrap()
            .to_string();

        let dst_file_path = cur_dir.clone().join("../testdata/dst-less-than-4k.dat");

        let src_file = File::open(&src_file_path).await?;
        let src_meta = src_file.metadata().await?;
        let src_spec = FileSpec {
            path: src_file_path.clone(),
            name: "src.dat".to_string(),
            size: src_meta.len(),
        };

        let append_file = File::open(&append_file_path).await?;
        let append_meta = append_file.metadata().await?;
        let append_spec = FileSpec {
            path: append_file_path.clone(),
            name: "append-less-than-4k.dat".to_string(),
            size: append_meta.len(),
        };

        encode(
            src_spec,
            append_spec,
            dst_file_path.as_path(),
            PASSWORD,
            pcb,
            ProgressCbArg::default(),
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_file_encode_more_than_4k() -> Result<()> {
        let cur_dir = env::current_dir()?;

        let src_file_path = cur_dir
            .clone()
            .join("../testdata/src.dat")
            .to_str()
            .unwrap()
            .to_string();

        let append_file_path = cur_dir
            .clone()
            .join("../testdata/append-more-than-4k.dat")
            .to_str()
            .unwrap()
            .to_string();

        let dst_file_path = cur_dir.clone().join("../testdata/dst-more-than-4k.dat");

        println!("{src_file_path}");

        let src_file = File::open(&src_file_path).await?;
        let src_meta = src_file.metadata().await?;
        let src_spec = FileSpec {
            path: src_file_path.clone(),
            name: "src.dat".to_string(),
            size: src_meta.len(),
        };

        let append_file = File::open(&append_file_path).await?;
        let append_meta = append_file.metadata().await?;
        let append_spec = FileSpec {
            path: append_file_path.clone(),
            name: "append-more-than-4k.dat".to_string(),
            size: append_meta.len(),
        };

        encode(
            src_spec,
            append_spec,
            dst_file_path.as_path(),
            PASSWORD,
            pcb,
            ProgressCbArg::default(),
        )
        .await?;

        Ok(())
    }
}
