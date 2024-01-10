use crate::slint_generatedAppWindow::{AppWindow, Logic};
use slint::ComponentHandle;

pub fn init(ui: &AppWindow) {
    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_handle_ok_cancel_dialog(move |handle_type, _handle_uuid| {
            let ui = ui_handle.unwrap();

            match handle_type.as_str() {
                "encode" => {
                    ui.global::<Logic>().invoke_cancel_encode();
                }
                "decode" => {
                    ui.global::<Logic>().invoke_cancel_decode();
                }
                _ => (),
            }
        });
}
