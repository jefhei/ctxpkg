// Clipboard — cross-platform clipboard access
pub fn copy_to_clipboard(_text: &str) -> Result<(), crate::error::CtxpkgError> {
    // TODO: implement with arboard
    Err(crate::error::CtxpkgError::ClipboardError(
        "Clipboard not available in headless environment. Use `ctxpkg pack --output context.md` instead."
            .to_string(),
    ))
}
