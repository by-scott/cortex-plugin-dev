use cortex_sdk::ToolRuntime;

pub fn namespace(runtime: &dyn ToolRuntime) -> String {
    let invocation = runtime.invocation();
    invocation
        .actor
        .as_deref()
        .or(invocation.session_id.as_deref())
        .unwrap_or("global")
        .to_string()
}

pub fn emit_step(runtime: &dyn ToolRuntime, step: &str) {
    runtime.emit_progress(step);
}

pub fn observe(runtime: &dyn ToolRuntime, source: &str, message: impl AsRef<str>) {
    runtime.emit_observer(Some(source), message.as_ref());
}
