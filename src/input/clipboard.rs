use agent_desktop_core::error::AdapterError;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE,
};
use windows::Win32::System::Ole::CF_UNICODETEXT;

pub fn get() -> Result<String, AdapterError> {
    unsafe {
        OpenClipboard(HWND::default())
            .map_err(|e| AdapterError::internal(format\!("OpenClipboard failed: {}", e)))?;

        let handle = GetClipboardData(CF_UNICODETEXT.0 as u32);
        let result = match handle {
            Ok(h) if \!h.0.is_null() => {
                let ptr = GlobalLock(std::mem::transmute(h.0)) as *const u16;
                if ptr.is_null() {
                    Err(AdapterError::internal("GlobalLock returned null"))
                } else {
                    let mut len = 0;
                    while *ptr.add(len) \!= 0 {
                        len += 1;
                    }
                    let slice = std::slice::from_raw_parts(ptr, len);
                    let text = String::from_utf16_lossy(slice);
                    GlobalUnlock(std::mem::transmute(h.0));
                    Ok(text)
                }
            }
            _ => Err(AdapterError::internal("No text on clipboard")),
        };

        let _ = CloseClipboard();
        result
    }
}

pub fn set(text: &str) -> Result<(), AdapterError> {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let byte_len = wide.len() * 2;

    unsafe {
        let hmem = GlobalAlloc(GMEM_MOVEABLE, byte_len)
            .map_err(|e| AdapterError::internal(format\!("GlobalAlloc failed: {}", e)))?;

        let ptr = GlobalLock(hmem) as *mut u16;
        if ptr.is_null() {
            return Err(AdapterError::internal("GlobalLock returned null"));
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
        GlobalUnlock(hmem);

        OpenClipboard(HWND::default())
            .map_err(|e| AdapterError::internal(format\!("OpenClipboard failed: {}", e)))?;
        let _ = EmptyClipboard();

        SetClipboardData(CF_UNICODETEXT.0 as u32, std::mem::transmute(hmem.0))
            .map_err(|e| {
                let _ = CloseClipboard();
                AdapterError::internal(format\!("SetClipboardData failed: {}", e))
            })?;

        let _ = CloseClipboard();
    }
    Ok(())
}

pub fn clear() -> Result<(), AdapterError> {
    unsafe {
        OpenClipboard(HWND::default())
            .map_err(|e| AdapterError::internal(format\!("OpenClipboard failed: {}", e)))?;
        let _ = EmptyClipboard();
        let _ = CloseClipboard();
    }
    Ok(())
}
