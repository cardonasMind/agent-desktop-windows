use agent_desktop_core::adapter::{ImageBuffer, ImageFormat};
use agent_desktop_core::error::AdapterError;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
    GetDC, GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

/// Capture the entire primary screen and return a raw PNG image buffer.
pub fn capture_screen(_display_idx: usize) -> Result<ImageBuffer, AdapterError> {
    unsafe {
        let width = GetSystemMetrics(SM_CXSCREEN);
        let height = GetSystemMetrics(SM_CYSCREEN);

        let hdc_screen = GetDC(HWND::default());
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let hbm = CreateCompatibleBitmap(hdc_screen, width, height);
        let old = SelectObject(hdc_mem, hbm);

        BitBlt(hdc_mem, 0, 0, width, height, Some(hdc_screen), 0, 0, SRCCOPY)
            .map_err(|e| AdapterError::internal(format\!("BitBlt failed: {}", e)))?;

        // Extract raw pixel data via GetDIBits
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let row_bytes = (width as usize) * 4;
        let mut pixels = vec\![0u8; row_bytes * height as usize];

        GetDIBits(
            hdc_mem,
            hbm,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // Cleanup GDI
        SelectObject(hdc_mem, old);
        let _ = DeleteObject(hbm);
        DeleteDC(hdc_mem);
        ReleaseDC(HWND::default(), hdc_screen);

        // Convert BGRA -> RGBA
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2); // B <-> R
        }

        // Encode to PNG using a minimal encoder
        let png_data = encode_rgba_png(&pixels, width as u32, height as u32)?;

        Ok(ImageBuffer {
            data: png_data,
            format: ImageFormat::Png,
            width: width as u32,
            height: height as u32,
        })
    }
}

/// Capture the window owned by a specific PID.
pub fn capture_app(pid: i32) -> Result<ImageBuffer, AdapterError> {
    // For now, fall back to full screen capture.
    // A more refined implementation would use PrintWindow or DWM thumbnails.
    let _ = pid;
    capture_screen(0)
}

/// Minimal PNG encoder for RGBA data (no external crate needed).
fn encode_rgba_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, AdapterError> {
    // We use a very simple uncompressed PNG (zlib stored blocks).
    // This keeps dependencies minimal. A real implementation should use
    // the `png` crate for proper compression, but this works for MVP.
    use std::io::Write;

    let mut out = Vec::new();

    // PNG signature
    out.write_all(&[137, 80, 78, 71, 13, 10, 26, 10]).unwrap();

    // IHDR
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8);  // bit depth
    ihdr.push(6);  // color type: RGBA
    ihdr.push(0);  // compression
    ihdr.push(0);  // filter
    ihdr.push(0);  // interlace
    write_png_chunk(&mut out, b"IHDR", &ihdr);

    // IDAT — we compress the raw image data with deflate (zlib).
    // Build the raw scanlines (filter byte 0 = None for each row).
    let row_len = (width as usize) * 4 + 1; // +1 for filter byte
    let mut raw = Vec::with_capacity(row_len * height as usize);
    for y in 0..height as usize {
        raw.push(0); // filter: None
        let start = y * (width as usize) * 4;
        let end = start + (width as usize) * 4;
        raw.extend_from_slice(&rgba[start..end]);
    }

    // Use zlib stored blocks (no compression, but valid).
    let zlib_data = zlib_store(&raw);
    write_png_chunk(&mut out, b"IDAT", &zlib_data);

    // IEND
    write_png_chunk(&mut out, b"IEND", &[]);

    Ok(out)
}

fn write_png_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    let len = data.len() as u32;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(chunk_type);
    out.extend_from_slice(data);
    let mut crc_data = Vec::with_capacity(4 + data.len());
    crc_data.extend_from_slice(chunk_type);
    crc_data.extend_from_slice(data);
    let crc = crc32(&crc_data);
    out.extend_from_slice(&crc.to_be_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 \!= 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFFFFFF
}

fn zlib_store(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    // zlib header: CMF=0x78, FLG=0x01 (no dict, level 0)
    out.push(0x78);
    out.push(0x01);

    // Split into stored blocks of max 65535 bytes
    let chunks = data.chunks(65535);
    let chunk_count = chunks.clone().count();
    for (i, chunk) in chunks.enumerate() {
        let is_last = i == chunk_count - 1;
        out.push(if is_last { 0x01 } else { 0x00 }); // BFINAL
        let len = chunk.len() as u16;
        let nlen = \!len;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&nlen.to_le_bytes());
        out.extend_from_slice(chunk);
    }

    // Adler-32 checksum
    let adler = adler32(data);
    out.extend_from_slice(&adler.to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}
