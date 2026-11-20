use slint::Model;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    let w = ui.as_weak();
    ui.on_send(move || {
        let ui = w.unwrap();
        let text = ui.get_draft().trim().to_string();
        if text.is_empty() {
            return;
        }
        let mut msgs: Vec<slint::SharedString> = ui.get_messages().iter().collect();
        msgs.push(format!("你：{text}").into());
        ui.set_messages(msgs.as_slice().into());
        ui.set_draft("".into());

        // 前缀指纹：同一串消息永远得到同一个值，用来判定「这段已经算过了」。
        let prefix: Vec<agent_core::Msg> = msgs
            .iter()
            .map(|s| agent_core::Msg {
                role: agent_core::Role::User,
                body: s.to_string(),
            })
            .collect();
        eprintln!("prefix {:016x}", agent_core::prefix_hash(&prefix));
    });

    ui.run()
}
