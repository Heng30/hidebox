use crate::file;
use crate::message::{async_message_success, async_message_warn};
use crate::message_warn;
use crate::slint_generatedAppWindow::{AppWindow, DecodeSpec, Logic, Store};
use crate::util::translator::tr;
use anyhow::Result;
use native_dialog::FileDialog;
use slint::{ComponentHandle, Weak};
use std::path::Path;
use tokio::fs::File;
use tokio::task::spawn;

pub fn init(ui: &AppWindow) {
    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_cancel_decode(move || {
        file::decode::cancel();

        let ui = ui_handle.unwrap();
        ui.global::<Store>().set_decode_spec(DecodeSpec::default());
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_load_decode_src_file(move || {
        let ui = ui_handle.unwrap();

        match FileDialog::new().set_location("~").show_open_single_file() {
            Ok(Some(file)) => {
                let file_path = file.to_str().unwrap().to_string();

                let ui = ui.as_weak();
                spawn(async move {
                    match inner_load_decode_src_file(&file_path).await {
                        Err(e) => {
                            async_message_warn(
                                ui.clone(),
                                format!("{}. {}: {e:?}", tr("出错"), tr("原因")),
                            );
                            return;
                        }
                        Ok(false) => {
                            async_message_warn(ui.clone(), tr("非法文件"));
                            return;
                        }
                        _ => (),
                    }

                    let _ = slint::invoke_from_event_loop(move || {
                        let ui = ui.clone().unwrap();
                        let mut spec = ui.global::<Store>().get_decode_spec();
                        spec.src_file = file_path.into();
                        ui.global::<Store>().set_decode_spec(spec);
                    });
                });
            }
            Err(e) => {
                message_warn!(&ui, format!("{}{:?}", tr("打开文件失败"), e));
            }
            _ => (),
        };
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_load_decode_dst_file(move || {
        let ui = ui_handle.unwrap();
        let output_file = ui.global::<Store>().get_decode_spec().src_file.to_string();

        match FileDialog::new()
            .set_location("~")
            .set_filename(&output_file)
            .show_save_single_file()
        {
            Ok(Some(file)) => {
                let mut spec = ui.global::<Store>().get_decode_spec();
                spec.dst_file = file.to_str().unwrap().into();
                ui.global::<Store>().set_decode_spec(spec);
            }
            Err(e) => {
                message_warn!(&ui, format!("{}{:?}", tr("打开文件失败"), e));
            }
            _ => (),
        };
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_decode(move |password| {
        let ui = ui_handle.unwrap();
        let password = password.to_string();

        let spec = ui.global::<Store>().get_decode_spec();
        let src_file_path = spec.src_file.to_string();
        let dst_file_path = spec.dst_file.to_string();

        if src_file_path.is_empty() || dst_file_path.is_empty() {
            message_warn!(&ui, tr("文件名为空"));
            return;
        }

        let ui = ui.as_weak();
        spawn(async move {
            match inner_decode(ui.clone(), src_file_path, dst_file_path, password).await {
                Ok(v) => async_message_success(ui.clone(), v),
                Err(e) => {
                    async_message_warn(ui.clone(), format!("{}. {}: {e:?}", tr("出错"), tr("原因")))
                }
            }
        });
    });
}

async fn inner_decode(
    ui: Weak<AppWindow>,
    src_file_path: String,
    dst_file_path: String,
    password: String,
) -> Result<String> {
    let src_file = File::open(&src_file_path).await?;
    let src_meta = src_file.metadata().await?;
    let src_name = Path::new(&src_file_path)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let src_spec = file::FileSpec {
        path: src_file_path,
        name: src_name,
        size: src_meta.len(),
    };

    file::decode(
        src_spec,
        Path::new(&dst_file_path),
        &password,
        pcb,
        file::ProgressCbArg {
            ui: Some(ui),
            ..Default::default()
        },
    )
    .await
}

async fn inner_load_decode_src_file(file_path: &str) -> Result<bool> {
    let file = File::open(&file_path).await?;
    let meta = file.metadata().await?;
    let spec = file::FileSpec {
        path: file_path.to_string(),
        name: String::default(),
        size: meta.len(),
    };

    file::decode::has_append_file(&spec).await
}

fn pcb(arg: file::ProgressCbArg) {
    let _ = slint::invoke_from_event_loop(move || {
        let ui = arg.ui.unwrap().unwrap();
        let mut spec = ui.global::<Store>().get_decode_spec();
        spec.progress = arg.progress as f32;
        ui.global::<Store>().set_decode_spec(spec);
    });
}
