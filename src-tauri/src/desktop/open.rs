//! Opening desktop items through the Windows shell (the way a double-click
//! on the real desktop would). We never implement custom file I/O here.

use windows::core::{HSTRING, PCWSTR};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::app::error::{AppError, AppResult};

/// Open a path with the shell's default verb ("open"). Folders open in
/// Explorer, files with their associated application, shortcuts get launched.
pub fn open_with_shell(path: &str) -> AppResult<()> {
    unsafe {
        let verb = HSTRING::from("open");
        let file = HSTRING::from(path);
        let result = ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
        // ShellExecuteW returns a pseudo-instance: values > 32 mean success,
        // small values are SE_ERR_* error codes.
        let code = result.0 as isize;
        if code <= 32 {
            return Err(AppError::Other(format!(
                "Windows 无法打开该项（错误码 {code}）"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn opening_a_missing_file_fails_with_shell_error() {
        let err = open_with_shell("Z:\\definitely\\missing\\item.xyz").unwrap_err();
        assert!(err.to_string().contains("错误码"), "{err}");
    }
}
