use crate::slint_generatedAppWindow::AppWindow;
use slint::Weak;

pub mod decode;
pub mod encode;

const CHUNK_SIZE: usize = 4096;
const CHUNK_LEN_SIZE: usize = 8;
const HASH_TEXT_SIZE: usize = 32;
const MIN_CHUNK_LEN: usize = CHUNK_LEN_SIZE + HASH_TEXT_SIZE;
const MAGIC_NUM: &str = "HIDEBOX";
const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024 * 1024; // 2G

#[derive(Clone, Debug, Default)]
pub struct FileSpec {
    pub path: String,
    pub name: String,
    pub size: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HideSpec {
    pub append_name: String,
    pub append_size: u64,
    pub src_size: u64,
}

#[derive(Clone, Default, Debug)]
struct ChunkSpec {
    pub data: Vec<u8>,
}

#[derive(Clone, Default)]
struct ProgressCbArg {
    pub progress: u32,
    pub ui: Option<Weak<AppWindow>>,
}

type ProgressCb = fn(ProgressCbArg);
