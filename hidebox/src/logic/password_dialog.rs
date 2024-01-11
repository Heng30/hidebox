use crate::slint_generatedAppWindow::{AppWindow, Logic};
use slint::ComponentHandle;

pub fn init(ui: &AppWindow) {
    let ui_handle = ui.as_weak();
    ui.global::<Logic>()
        .on_handle_password_dialog(move |handle_type, _handle_uuid, password| {
            let ui = ui_handle.unwrap();

            match handle_type.as_str() {
                "encode" => {
                    ui.global::<Logic>().invoke_encode(password);
                }
                "decode" => {
                    ui.global::<Logic>().invoke_decode(password);
                }
                _ => (),
            }
        });
}
