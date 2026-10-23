use std::{mem::size_of, ptr::null, sync::OnceLock};
use windows_sys::Win32::{
    Foundation::HWND,
    Graphics::Dwm::{DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        DestroyIcon, GCLP_HICON, GCLP_HICONSM, GetClassLongPtrW, GetSystemMetrics, ICON_BIG,
        ICON_SMALL, IMAGE_ICON, LoadImageW, SM_CXICON, SM_CXSMICON, SM_CYICON, SM_CYSMICON,
        SendMessageW, SetClassLongPtrW, WM_SETICON,
    },
};

pub fn set_application_icons(window: usize) -> bool {
    // Class icons can outlive a window; these two cached handles live until process exit.
    static ICONS: OnceLock<Option<(usize, usize)>> = OnceLock::new();
    let Some(&(large, small)) = ICONS
        .get_or_init(|| unsafe {
            let module = GetModuleHandleW(null());
            let large = LoadImageW(
                module,
                std::ptr::without_provenance(1),
                IMAGE_ICON,
                GetSystemMetrics(SM_CXICON),
                GetSystemMetrics(SM_CYICON),
                0,
            );
            let small = LoadImageW(
                module,
                std::ptr::without_provenance(1),
                IMAGE_ICON,
                GetSystemMetrics(SM_CXSMICON),
                GetSystemMetrics(SM_CYSMICON),
                0,
            );
            if large.is_null() || small.is_null() {
                if !large.is_null() {
                    DestroyIcon(large);
                }
                if !small.is_null() {
                    DestroyIcon(small);
                }
                None
            } else {
                Some((large as usize, small as usize))
            }
        })
        .as_ref()
    else {
        return false;
    };
    let window = window as HWND;
    unsafe {
        SetClassLongPtrW(window, GCLP_HICON, large as isize);
        SetClassLongPtrW(window, GCLP_HICONSM, small as isize);
        SendMessageW(window, WM_SETICON, ICON_BIG as usize, large as isize);
        SendMessageW(window, WM_SETICON, ICON_SMALL as usize, small as isize);
        GetClassLongPtrW(window, GCLP_HICON) == large
            && GetClassLongPtrW(window, GCLP_HICONSM) == small
    }
}

pub fn round_corners(window: usize) -> bool {
    let preference = DWMWCP_ROUND;
    unsafe {
        DwmSetWindowAttribute(
            window as HWND,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            (&preference as *const i32).cast(),
            size_of::<i32>() as u32,
        ) >= 0
    }
}

pub fn round_owned_window(title: &str) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowThreadProcessId};
    let title: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let window = FindWindowW(std::ptr::null(), title.as_ptr());
        let mut process = 0;
        GetWindowThreadProcessId(window, &mut process);
        !window.is_null() && process == std::process::id() && round_corners(window as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::{Win32::UI::WindowsAndMessaging::*, core::w};

    #[test]
    fn application_logo_is_assigned_to_native_class_and_window_icons() {
        unsafe {
            let instance = GetModuleHandleW(null());
            let class_name = w!("ReebWindowIconProbe");
            let class = WNDCLASSW {
                lpfnWndProc: Some(DefWindowProcW),
                hInstance: instance,
                lpszClassName: class_name,
                ..Default::default()
            };
            assert_ne!(RegisterClassW(&class), 0);
            let window = CreateWindowExW(
                0,
                class_name,
                w!("Reeb icon probe"),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                100,
                100,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                null(),
            );
            assert!(!window.is_null());
            assert_eq!(IsWindowVisible(window), 0);
            assert_eq!(GetClassLongPtrW(window, GCLP_HICON), 0);
            let assigned = set_application_icons(window as usize);
            let large = GetClassLongPtrW(window, GCLP_HICON);
            let small = GetClassLongPtrW(window, GCLP_HICONSM);
            let window_large = SendMessageW(window, WM_GETICON, ICON_BIG as usize, 0);
            let window_small = SendMessageW(window, WM_GETICON, ICON_SMALL as usize, 0);
            DestroyWindow(window);
            UnregisterClassW(class_name, instance);
            assert!(assigned);
            assert_ne!(large, 0);
            assert_ne!(small, 0);
            assert_eq!(window_large as usize, large);
            assert_eq!(window_small as usize, small);
        }
    }
}
