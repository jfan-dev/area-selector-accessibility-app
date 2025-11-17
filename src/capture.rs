use image::{Rgba, RgbaImage};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;

pub fn capture_area(x: u32, y: u32, w: u32, h: u32) -> Option<RgbaImage> {
    unsafe {
        let hdc_screen = GetDC(HWND(std::ptr::null_mut()));
        if hdc_screen.0.is_null() {
            return None;
        }

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        if hdc_mem.0.is_null() {
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
            return None;
        }

        let hbitmap = CreateCompatibleBitmap(hdc_screen, w as i32, h as i32);
        if hbitmap.0.is_null() {
            DeleteDC(hdc_mem);
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
            return None;
        }

        let old_obj = SelectObject(hdc_mem, HGDIOBJ(hbitmap.0));
        if old_obj.0.is_null() {
            DeleteObject(HGDIOBJ(hbitmap.0));
            DeleteDC(hdc_mem);
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
            return None;
        }

        if BitBlt(
            hdc_mem,
            0,
            0,
            w as i32,
            h as i32,
            hdc_screen,
            x as i32,
            y as i32,
            SRCCOPY,
        ).is_ok()
        {
            let mut bmp_info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w as i32,
                    biHeight: -(h as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0 as u32,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [RGBQUAD {
                    rgbBlue: 0,
                    rgbGreen: 0,
                    rgbRed: 0,
                    rgbReserved: 0,
                }; 1],
            };

            let buf_size = (w * h * 4) as usize;
            let mut buffer = vec![0u8; buf_size];

            if GetDIBits(
                hdc_mem,
                hbitmap,
                0,
                h as u32,
                Some(buffer.as_mut_ptr() as *mut _),
                &mut bmp_info,
                DIB_RGB_COLORS,
            ) != 0
            {
                let mut img = RgbaImage::new(w, h);

                for iy in 0..h {
                    for ix in 0..w {
                        let i = ((iy * w + ix) * 4) as usize;
                        if i + 3 >= buffer.len() {
                            continue;
                        }
                        img.put_pixel(
                            ix,
                            iy,
                            Rgba([
                                buffer[i + 2],
                                buffer[i + 1],
                                buffer[i],
                                255,
                            ]),
                        );
                    }
                }

                SelectObject(hdc_mem, old_obj);
                DeleteObject(HGDIOBJ(hbitmap.0));
                DeleteDC(hdc_mem);
                ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);

                return Some(img);
            }
        }

        SelectObject(hdc_mem, old_obj);
        DeleteObject(HGDIOBJ(hbitmap.0));
        DeleteDC(hdc_mem);
        ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);

        None
    }
}
