use super::*;
use crate::{util, util::translator::tr};
use anyhow::{anyhow, Result};
use std::io::SeekFrom;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

pub static CANCEL_DECODE: AtomicBool = AtomicBool::new(false);

pub fn cancel() {
    CANCEL_DECODE.store(true, Ordering::SeqCst);
}

#[allow(dead_code)]
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

pub async fn has_append_file(file_spec: &FileSpec) -> Result<bool> {
    if file_spec.size <= MAGIC_NUM.len() as u64 {
        return Ok(false);
    }

    let mut magic_buf = vec![0_u8; MAGIC_NUM.len()];
    let mut file = File::open(&file_spec.path).await?;
    file.seek(SeekFrom::Start(file_spec.size - MAGIC_NUM.len() as u64))
        .await?;

    file.read_exact(&mut magic_buf).await?;

    Ok(magic_buf == MAGIC_NUM.as_bytes())
}

async fn get_hide_spec_data(file_spec: &FileSpec, password: &str) -> Result<HideSpec> {
    let pos_of_end = CHUNK_LEN_SIZE + MAGIC_NUM.len();

    if file_spec.size <= pos_of_end as u64 {
        return Err(anyhow!("do not contain hide specify data"));
    }

    let mut chunk_len_buf = vec![0_u8; CHUNK_LEN_SIZE];
    let mut file = File::open(&file_spec.path).await?;
    file.seek(SeekFrom::Start(file_spec.size - pos_of_end as u64))
        .await?;

    file.read_exact(&mut chunk_len_buf).await?;

    let chunk_len_str = String::from_utf8_lossy(&chunk_len_buf).trim().to_string();
    let chunk_len = usize::from_str_radix(&chunk_len_str, 16)?;

    if chunk_len > CHUNK_SIZE {
        return Err(anyhow!("invalid hide specify lenght"));
    }

    let mut hide_spec_data = vec![0; chunk_len];
    file.seek(SeekFrom::Start(
        file_spec.size - (pos_of_end + chunk_len) as u64,
    ))
    .await?;

    file.read_exact(&mut hide_spec_data).await?;
    let hide_spec_data = String::from_utf8_lossy(&hide_spec_data);
    let hide_spec_data = util::crypto::decrypt(password, &hide_spec_data)?;
    let hide_spec_data = String::from_utf8_lossy(&hide_spec_data);

    match serde_json::from_str(&hide_spec_data) {
        Ok(v) => Ok(v),
        Err(_) => Err(anyhow!("wrong password: {password}")),
    }
}

pub async fn decode(
    src_file_spec: FileSpec,
    output_file: &Path,
    password: &str,
    progress_callback: ProgressCb,
    mut progress_callback_arg: ProgressCbArg,
) -> Result<String> {
    CANCEL_DECODE.store(false, Ordering::SeqCst);

    let hide_spec = get_hide_spec_data(&src_file_spec, password).await?;

    let mut src_file = File::open(&src_file_spec.path).await?;
    let mut output_file = File::create(&output_file).await?;

    src_file.seek(SeekFrom::Start(hide_spec.src_size)).await?;

    let mut magic_buf = vec![0_u8; MAGIC_NUM.len()];
    src_file.read_exact(&mut magic_buf).await?;
    if magic_buf != MAGIC_NUM.as_bytes() {
        return Err(anyhow!("do not find magic number before append file"));
    }

    let mut current = 0;
    let mut total_chunks = 0;

    loop {
        let mut chunk_len_buf = vec![0; CHUNK_LEN_SIZE];
        src_file.read_exact(&mut chunk_len_buf).await?;
        let chunk_len_str = String::from_utf8_lossy(&chunk_len_buf).trim().to_string();
        let chunk_len = usize::from_str_radix(&chunk_len_str, 16)?;

        if chunk_len > CHUNK_SIZE * 4 {
            return Err(anyhow!(
                "invalid chunk length, it is larger than {}",
                CHUNK_SIZE * 4
            ));
        }

        let mut encrypt_buf = vec![0; chunk_len];
        src_file.read_exact(&mut encrypt_buf).await?;

        let mut chunk_buf = Vec::with_capacity(CHUNK_LEN_SIZE + chunk_len);
        chunk_buf.append(&mut chunk_len_buf);
        chunk_buf.append(&mut encrypt_buf);

        let chunk_spec = parse_chunk(password, &chunk_buf)?;
        output_file.write_all(&chunk_spec.data).await?;

        total_chunks += 1;
        current += (CHUNK_LEN_SIZE + chunk_len) as u64;

        if CANCEL_DECODE.load(Ordering::SeqCst) {
            return Ok(tr("取消成功"));
        }

        if total_chunks % 10 == 0 {
            let progress = ((current as f64 / hide_spec.append_size as f64) * 100.) as u32;
            progress_callback_arg.progress = progress;
            progress_callback(progress_callback_arg.clone());
            // log::debug!(
            //     "current={} total={} progress={progress}",
            //     current,
            //     hide_spec.append_size
            // );
        }

        if current >= hide_spec.append_size {
            break;
        }
    }

    progress_callback_arg.progress = 100;
    progress_callback(progress_callback_arg);

    Ok(tr("解码成功"))
}

#[cfg(test)]
mod tests {
    use super::super::encode::make_chunk;
    use super::*;
    use std::env;

    const PASSWORD: &str = "123456";

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

    #[tokio::test]
    async fn test_file_has_append_file() -> Result<()> {
        let cur_dir = env::current_dir()?;

        let src_file_path = cur_dir
            .clone()
            .join("../testdata/src.dat")
            .to_str()
            .unwrap()
            .to_string();

        let src_file = File::open(&src_file_path).await?;
        let src_meta = src_file.metadata().await?;
        let src_spec = FileSpec {
            path: src_file_path.clone(),
            name: "src.dat".to_string(),
            size: src_meta.len(),
        };

        assert_eq!(has_append_file(&src_spec).await?, false);

        let dst_file_path = cur_dir
            .clone()
            .join("../testdata/dst-less-than-4k.dat")
            .to_str()
            .unwrap()
            .to_string();

        let dst_file = File::open(&dst_file_path).await?;
        let dst_meta = dst_file.metadata().await?;
        let dst_spec = FileSpec {
            path: dst_file_path.clone(),
            name: "dst-less-than-4k.dat".to_string(),
            size: dst_meta.len(),
        };

        assert_eq!(has_append_file(&dst_spec).await?, true);

        Ok(())
    }

    #[tokio::test]
    async fn test_file_get_hide_spec_data() -> Result<()> {
        let cur_dir = env::current_dir()?;

        let src_file_path = cur_dir
            .clone()
            .join("../testdata/src.dat")
            .to_str()
            .unwrap()
            .to_string();

        let src_file = File::open(&src_file_path).await?;
        let src_meta = src_file.metadata().await?;
        let src_spec = FileSpec {
            path: src_file_path.clone(),
            name: "src.dat".to_string(),
            size: src_meta.len(),
        };

        assert!(get_hide_spec_data(&src_spec, PASSWORD).await.is_err());

        let dst_file_path = cur_dir
            .clone()
            .join("../testdata/dst-less-than-4k.dat")
            .to_str()
            .unwrap()
            .to_string();

        let dst_file = File::open(&dst_file_path).await?;
        let dst_meta = dst_file.metadata().await?;
        let dst_spec = FileSpec {
            path: dst_file_path.clone(),
            name: "dst-less-than-4k.dat".to_string(),
            size: dst_meta.len(),
        };

        let file_spec = get_hide_spec_data(&dst_spec, PASSWORD).await?;
        println!("{file_spec:?}");
        assert!(file_spec.src_size > 0);
        assert!(file_spec.append_size > 0);
        assert_eq!(file_spec.append_name, "append-less-than-4k.dat");

        Ok(())
    }

    fn pcb(arg: ProgressCbArg) {
        println!("progress: {}", arg.progress);
    }

    #[tokio::test]
    async fn test_file_decode_less_than_4k() -> Result<()> {
        let cur_dir = env::current_dir()?;

        let src_file_path = cur_dir
            .clone()
            .join("../testdata/dst-less-than-4k.dat")
            .to_str()
            .unwrap()
            .to_string();

        let src_file = File::open(&src_file_path).await?;
        let src_meta = src_file.metadata().await?;
        let src_spec = FileSpec {
            path: src_file_path.clone(),
            name: "dst-less-than-4k.dat".to_string(),
            size: src_meta.len(),
        };

        let output_file_path = cur_dir
            .clone()
            .join("../testdata/dst-less-than-4k-decode.dat");

        decode(
            src_spec,
            output_file_path.as_path(),
            PASSWORD,
            pcb,
            ProgressCbArg::default(),
        )
        .await?;

        let src_string =
            tokio::fs::read(cur_dir.clone().join("../testdata/append-less-than-4k.dat")).await?;
        let dst_string = tokio::fs::read(output_file_path).await?;
        assert_eq!(src_string, dst_string);

        Ok(())
    }

    #[tokio::test]
    async fn test_file_decode_more_than_4k() -> Result<()> {
        let cur_dir = env::current_dir()?;

        let src_file_path = cur_dir
            .clone()
            .join("../testdata/dst-more-than-4k.dat")
            .to_str()
            .unwrap()
            .to_string();

        let src_file = File::open(&src_file_path).await?;
        let src_meta = src_file.metadata().await?;
        let src_spec = FileSpec {
            path: src_file_path.clone(),
            name: "dst-more-than-4k.dat".to_string(),
            size: src_meta.len(),
        };

        let output_file_path = cur_dir
            .clone()
            .join("../testdata/dst-more-than-4k-decode.dat");

        decode(
            src_spec,
            output_file_path.as_path(),
            PASSWORD,
            pcb,
            ProgressCbArg::default(),
        )
        .await?;

        let src_string =
            tokio::fs::read(cur_dir.clone().join("../testdata/append-more-than-4k.dat")).await?;
        let dst_string = tokio::fs::read(output_file_path).await?;
        assert_eq!(src_string, dst_string);

        Ok(())
    }
}
