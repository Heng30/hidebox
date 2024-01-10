use crate::message::{async_message_success, async_message_warn};
use crate::message_warn;
use crate::slint_generatedAppWindow::{AppWindow, EncodeSpec, Logic, Store};
use crate::util;
use crate::util::translator::tr;
use native_dialog::FileDialog;
use slint::{ComponentHandle, Weak};
use tokio::task::spawn;

pub fn init(ui: &AppWindow) {
    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_cancel_encode(move || {
        let ui = ui_handle.unwrap();
        ui.global::<Store>().set_encode_spec(EncodeSpec::default());
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_load_encode_src_file(move || {
        let ui = ui_handle.unwrap();

        // write data to end of these files can not effect file format
        match FileDialog::new()
            .set_location("~")
            .add_filter(
                "Image",
                &[
                    "bmp", "png", "jpg", "gif", "exe", "pdf", "jar", "rar", "mp4",
                ],
            )
            .show_open_single_file()
        {
            Ok(Some(file)) => {
                let mut spec = ui.global::<Store>().get_encode_spec();
                spec.src_file = file.to_str().unwrap().into();
                ui.global::<Store>().set_encode_spec(spec);
            }
            Err(e) => {
                message_warn!(&ui, format!("{}{:?}", tr("打开文件失败"), e));
            }
            _ => (),
        };
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_load_encode_append_file(move || {
        let ui = ui_handle.unwrap();

        match FileDialog::new().set_location("~").show_open_single_file() {
            Ok(Some(file)) => {
                let mut spec = ui.global::<Store>().get_encode_spec();
                spec.append_file = file.to_str().unwrap().into();
                ui.global::<Store>().set_encode_spec(spec);
            }
            Err(e) => {
                message_warn!(&ui, format!("{}{:?}", tr("打开文件失败"), e));
            }
            _ => (),
        };
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_load_encode_dst_file(move || {
        let ui = ui_handle.unwrap();
        let output_file = format!("output-{}", ui.global::<Store>().get_encode_spec().src_file);

        match FileDialog::new()
            .set_location("~")
            .set_filename(&output_file)
            .show_save_single_file()
        {
            Ok(Some(file)) => {
                let mut spec = ui.global::<Store>().get_encode_spec();
                spec.dst_file = file.to_str().unwrap().into();
                ui.global::<Store>().set_encode_spec(spec);
            }
            Err(e) => {
                message_warn!(&ui, format!("{}{:?}", tr("打开文件失败"), e));
            }
            _ => (),
        };
    });

    let ui_handle = ui.as_weak();
    ui.global::<Logic>().on_encode(move |password| {
        let password = password.to_string();
        let ui = ui_handle.clone();
        spawn(async move {
            let _ = slint::invoke_from_event_loop(move || {
                let ui = ui.clone().unwrap();
                // let mut account = ui.global::<Store>().get_account();
                // ui.global::<Store>().set_account(account);
            });
        });
    });
}

//     ui.global::<Logic>()
//         .on_new_mnemonic(move || AddressInfo::mnemonic().into());

//     ui.global::<Logic>()
//         .on_mnemonic_word(move |mnemonic, index| {
//             let index = index as usize;
//             let items: Vec<&str> = mnemonic.as_str().split_whitespace().collect();
//             if index < items.len() {
//                 items[index].into()
//             } else {
//                 slint::SharedString::default()
//             }
//         });

//     let ui_handle = ui.as_weak();
//     ui.global::<Logic>()
//         .on_switch_network(move |uuid, network| {
//             let ui = ui_handle.clone();
//             let uuid = uuid.to_string();
//             let network = network.to_string();

//             spawn(async move {
//                 match db::account::select(&uuid).await {
//                     Err(e) => async_message_warn(
//                         ui.clone(),
//                         format!("{}. {}: {e:?}", tr("出错"), tr("原因")),
//                     ),
//                     Ok(account) => match serde_json::from_str::<Value>(&account.data) {
//                         Err(e) => async_message_warn(
//                             ui.clone(),
//                             format!("{}. {}: {e:?}", tr("出错"), tr("原因")),
//                         ),
//                         Ok(mut value) => {
//                             value["network"] = Value::String(network.clone());
//                             let address = if network == "main" {
//                                 value["main-address"].as_str().unwrap().to_string()
//                             } else {
//                                 value["test-address"].as_str().unwrap().to_string()
//                             };

//                             match db::account::update(&uuid, &value.to_string()).await {
//                                 Err(e) => {
//                                     async_message_warn(
//                                         ui.clone(),
//                                         format!("{}. {}: {e:?}", tr("出错"), tr("原因")),
//                                     );
//                                     return;
//                                 }
//                                 _ => async_message_success(ui.clone(), tr("切换网络成功")),
//                             }

//                             super::activity::load_items(ui.clone(), network.clone());
//                             super::address_book::load_items(ui.clone(), network.clone());

//                             let _ = slint::invoke_from_event_loop(move || {
//                                 let ui = ui.clone().unwrap();
//                                 let mut account = ui.global::<Store>().get_account();
//                                 account.network = network.into();
//                                 account.address = address.into();
//                                 account.balance_btc = "0".into();
//                                 account.balance_usd = "0".into();
//                                 ui.global::<Store>().set_account(account);
//                             });
//                         }
//                     },
//                 }
//             });
//         });

//     let ui_handle = ui.as_weak();
//     ui.global::<Logic>()
//         .on_delete_account(move |uuid, password| {
//             let uuid = uuid.to_string();
//             let password = password.to_string();

//             let ui = ui_handle.clone();
//             spawn(async move {
//                 if !is_password_verify(uuid.clone(), password).await {
//                     async_message_warn(ui.clone(), tr("密码错误"));
//                     return;
//                 }

//                 let _ = db::account::delete(&uuid).await;
//                 let _ = db::activity::delete_all().await;

//                 let _ = slint::invoke_from_event_loop(move || {
//                     let ui = ui.clone().unwrap();

//                     ui.global::<Store>()
//                         .get_activity_datas()
//                         .as_any()
//                         .downcast_ref::<VecModel<ActivityItem>>()
//                         .expect("We know we set a VecModel earlier")
//                         .set_vec(vec![]);

//                     let account = SAccount {
//                         balance_btc: "0".into(),
//                         balance_usd: "0".into(),
//                         ..Default::default()
//                     };
//                     ui.global::<Store>().set_account(account);

//                     ui.global::<Store>()
//                         .set_account_mnemonic(AddressInfo::mnemonic().into());
//                     ui.set_new_account_dialog_type_index(0);
//                     ui.global::<Store>().set_is_show_new_account_dialog(true);

//                     ui.global::<Logic>()
//                         .invoke_show_message(tr("删除成功").into(), "success".into());
//                 });
//             });
//         });

//     let ui_handle = ui.as_weak();
//     ui.global::<Logic>()
//         .on_show_mnemonic(move |uuid, password| {
//             let uuid = uuid.to_string();
//             let password = password.to_string();

//             let ui = ui_handle.clone();
//             spawn(async move {
//                 if !is_password_verify(uuid.clone(), password.clone()).await {
//                     async_message_warn(ui.clone(), tr("密码错误"));
//                     return;
//                 }

//                 match db::account::select(&uuid).await {
//                     Err(e) => async_message_warn(
//                         ui.clone(),
//                         format!("{}. {}: {e:?}", tr("出错"), tr("原因")),
//                     ),
//                     Ok(account) => match serde_json::from_str::<Value>(&account.data) {
//                         Err(e) => async_message_warn(
//                             ui.clone(),
//                             format!("{}. {}: {e:?}", tr("出错"), tr("原因")),
//                         ),
//                         Ok(value) => {
//                             let mnemonic = value["mnemonic"].as_str().unwrap();
//                             let mnemonic = util::crypto::decrypt(&password, mnemonic).unwrap();
//                             let mnemonic = String::from_utf8_lossy(&mnemonic).to_string();
//                             let _ = slint::invoke_from_event_loop(move || {
//                                 let ui = ui.clone().unwrap();
//                                 ui.global::<Store>().set_account_mnemonic(mnemonic.into());
//                                 ui.global::<Store>().set_is_show_show_mnemonic_dialog(true);
//                             });
//                         }
//                     },
//                 }
//             });
//         });

//     let ui_handle = ui.as_weak();
//     ui.global::<Logic>()
//         .on_recover_account(move |uuid, password| {
//             let uuid = uuid.to_string();
//             let password = password.to_string();

//             let ui = ui_handle.clone();
//             spawn(async move {
//                 if !is_password_verify(uuid.clone(), password.clone()).await {
//                     async_message_warn(ui.clone(), tr("密码错误"));
//                     return;
//                 }

//                 let _ = db::account::delete(&uuid).await;
//                 let _ = db::activity::delete_all().await;

//                 let _ = slint::invoke_from_event_loop(move || {
//                     let ui = ui.clone().unwrap();

//                     ui.global::<Store>()
//                         .get_activity_datas()
//                         .as_any()
//                         .downcast_ref::<VecModel<ActivityItem>>()
//                         .expect("We know we set a VecModel earlier")
//                         .set_vec(vec![]);

//                     let account = SAccount {
//                         balance_btc: "0".into(),
//                         balance_usd: "0".into(),
//                         ..Default::default()
//                     };
//                     ui.global::<Store>().set_account(account);

//                     ui.set_new_account_dialog_type_index(1);
//                     ui.global::<Store>().set_is_show_new_account_dialog(true);
//                 });
//             });
//         });

//     let ui_handle = ui.as_weak();
//     ui.global::<Logic>()
//         .on_change_password(move |uuid, old_password, new_password| {
//             let uuid = uuid.to_string();
//             let old_password = old_password.to_string();
//             let new_password = new_password.to_string();

//             let ui = ui_handle.clone();
//             spawn(async move {
//                 if !is_password_verify(uuid.clone(), old_password.clone()).await {
//                     async_message_warn(ui.clone(), tr("密码错误"));
//                     return;
//                 }

//                 match db::account::select(&uuid).await {
//                     Err(e) => async_message_warn(
//                         ui.clone(),
//                         format!("{}. {}: {e:?}", tr("出错"), tr("原因")),
//                     ),
//                     Ok(account) => match serde_json::from_str::<Value>(&account.data) {
//                         Err(e) => async_message_warn(
//                             ui.clone(),
//                             format!("{}. {}: {e:?}", tr("出错"), tr("原因")),
//                         ),
//                         Ok(mut value) => {
//                             let mnemonic = value["mnemonic"].as_str().unwrap();
//                             let mnemonic = util::crypto::decrypt(&old_password, mnemonic).unwrap();
//                             let mnemonic = util::crypto::encrypt(&new_password, &mnemonic).unwrap();
//                             value["password"] = Value::String(util::crypto::hash(&new_password));
//                             value["mnemonic"] = Value::String(mnemonic);
//                             match db::account::update(&uuid, &value.to_string()).await {
//                                 Err(e) => async_message_warn(
//                                     ui.clone(),
//                                     format!("{}. {}: {e:?}", tr("出错"), tr("原因")),
//                                 ),
//                                 _ => async_message_success(ui.clone(), tr("修改密码成功")),
//                             }
//                         }
//                     },
//                 }
//             });
//         });
// }

// fn load_items(ui: Weak<AppWindow>) {
//     spawn(async move {
//         match db::account::select_all().await {
//             Ok(items) => {
//                 if items.is_empty() {
//                     let _ = slint::invoke_from_event_loop(move || {
//                         let ui = ui.clone().unwrap();

//                         ui.global::<Store>()
//                             .set_account_mnemonic(AddressInfo::mnemonic().into());

//                         ui.global::<Store>().set_is_show_new_account_dialog(true);
//                     });
//                     return;
//                 }

//                 match serde_json::from_str::<Value>(&items[0].data) {
//                     Err(e) => {
//                         log::warn!("Error: {e:?}");
//                         let _ = slint::invoke_from_event_loop(move || {
//                             let ui = ui.clone().unwrap();
//                             ui.global::<Store>()
//                                 .set_account_mnemonic(AddressInfo::mnemonic().into());

//                             ui.global::<Store>().set_is_show_new_account_dialog(true);
//                         });
//                     }
//                     Ok(value) => {
//                         let uuid = value["uuid"].as_str().unwrap().to_string();
//                         let name = value["name"].as_str().unwrap().to_string();
//                         let network = value["network"].as_str().unwrap().to_string();
//                         let address = if network == "main" {
//                             value["main-address"].as_str().unwrap().to_string()
//                         } else {
//                             value["test-address"].as_str().unwrap().to_string()
//                         };

//                         super::activity::load_items(ui.clone(), network.clone());
//                         super::address_book::load_items(ui.clone(), network.clone());

//                         let _ = slint::invoke_from_event_loop(move || {
//                             let ui = ui.clone().unwrap();

//                             let mut account = ui.global::<Store>().get_account();
//                             account.uuid = uuid.into();
//                             account.name = name.into();
//                             account.network = network.into();
//                             account.address = address.into();
//                             ui.global::<Store>().set_account(account);

//                             ui.global::<Logic>().invoke_logout();
//                         });
//                     }
//                 }
//             }
//             Err(e) => log::warn!("Error: {}", e),
//         }
//     });
// }
