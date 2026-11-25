//! AppKit file destinations for Rune's overlay panel. Its window host only forwards
//! pointer input, so file drops must be delivered separately on the main thread.
use std::path::PathBuf;

pub enum Event {
    Hover(bool),
    Drop(Vec<PathBuf>),
}

#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, Sel};
    use objc2::{Encode, Encoding, class, msg_send, sel};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::ffi::{CStr, CString};

    #[link(name = "AppKit", kind = "framework")]
    unsafe extern "C" {
        static NSPasteboardNameDrag: *mut AnyObject;
    }

    #[derive(Default)]
    struct Destination {
        rect: [f32; 4],
        approach_width: f32,
        hovering: bool,
        disabled: bool,
        pasteboard_count: isize,
        dragging_files: bool,
        events: Vec<Event>,
    }

    thread_local! {
        static DESTINATIONS: RefCell<HashMap<usize, Destination>> = RefCell::default();
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Point {
        x: f64,
        y: f64,
    }
    unsafe impl Encode for Point {
        const ENCODING: Encoding = Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Size {
        w: f64,
        h: f64,
    }
    unsafe impl Encode for Size {
        const ENCODING: Encoding = Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Rect {
        origin: Point,
        size: Size,
    }
    unsafe impl Encode for Rect {
        const ENCODING: Encoding = Encoding::Struct("CGRect", &[Point::ENCODING, Size::ENCODING]);
    }

    unsafe fn string(value: *mut AnyObject) -> Option<String> {
        if value.is_null() {
            return None;
        }
        let utf8: *const std::ffi::c_char = unsafe { msg_send![value, UTF8String] };
        if utf8.is_null() {
            return None;
        }
        Some(
            unsafe { CStr::from_ptr(utf8) }
                .to_string_lossy()
                .into_owned(),
        )
    }

    unsafe fn paths(info: *mut AnyObject) -> Vec<PathBuf> {
        unsafe {
            let pb: *mut AnyObject = msg_send![info, draggingPasteboard];
            pasteboard_paths(pb)
        }
    }

    unsafe fn pasteboard_paths(pb: *mut AnyObject) -> Vec<PathBuf> {
        unsafe {
            let items: *mut AnyObject = msg_send![pb, pasteboardItems];
            if items.is_null() {
                return Vec::new();
            }
            let count: usize = msg_send![items, count];
            let kind: *mut AnyObject =
                msg_send![class!(NSString), stringWithUTF8String: c"public.file-url".as_ptr()];
            let mut files = Vec::new();
            for index in 0..count {
                let item: *mut AnyObject = msg_send![items, objectAtIndex: index];
                let value: *mut AnyObject = msg_send![item, stringForType: kind];
                if value.is_null() {
                    continue;
                }
                let url: *mut AnyObject = msg_send![class!(NSURL), URLWithString: value];
                if url.is_null() {
                    continue;
                }
                let file_url: *mut AnyObject = msg_send![url, filePathURL];
                if file_url.is_null() {
                    continue;
                }
                if let Some(path) = string(msg_send![file_url, path]) {
                    files.push(path.into());
                }
            }
            files
        }
    }

    unsafe extern "C" fn updated(view: &AnyObject, _: Sel, info: *mut AnyObject) -> usize {
        unsafe {
            let location: Point = msg_send![info, draggingLocation];
            let window: *mut AnyObject = msg_send![view, window];
            let frame: Rect = msg_send![window, frame];
            let x = location.x as f32;
            let y = (frame.size.h - location.y) as f32;
            let mask: usize = msg_send![info, draggingSourceOperationMask];
            let has_files = mask & 1 != 0 && !paths(info).is_empty();
            DESTINATIONS.with(|destinations| {
                let mut destinations = destinations.borrow_mut();
                let Some(d) = destinations.get_mut(&(view as *const _ as usize)) else {
                    return 0;
                };
                let [left, top, width, height] = d.rect;
                let over = !d.disabled
                    && has_files
                    && x >= left
                    && x <= left + width
                    && y >= top
                    && y <= top + height;
                d.dragging_files |= over;
                if over != d.hovering {
                    d.hovering = over;
                    d.events.push(Event::Hover(over));
                }
                usize::from(over)
            })
        }
    }

    unsafe extern "C" fn exited(view: &AnyObject, _: Sel, _: *mut AnyObject) {
        DESTINATIONS.with(|destinations| {
            if let Some(d) = destinations
                .borrow_mut()
                .get_mut(&(view as *const _ as usize))
            {
                d.hovering = false;
                d.events.push(Event::Hover(false));
            }
        });
    }

    unsafe extern "C" fn prepare(view: &AnyObject, selector: Sel, info: *mut AnyObject) -> Bool {
        Bool::new(unsafe { updated(view, selector, info) } != 0)
    }

    unsafe extern "C" fn perform(view: &AnyObject, selector: Sel, info: *mut AnyObject) -> Bool {
        if unsafe { updated(view, selector, info) } == 0 {
            return Bool::NO;
        }
        let files = unsafe { paths(info) };
        DESTINATIONS.with(|destinations| {
            if let Some(d) = destinations
                .borrow_mut()
                .get_mut(&(view as *const _ as usize))
            {
                d.hovering = false;
                d.events.push(Event::Drop(files));
            }
        });
        Bool::YES
    }

    pub fn install(window_number: u32) -> Option<usize> {
        // The panel and its view outlive the island. No ivars are added to the subclass.
        unsafe {
            let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
            let window: *mut AnyObject =
                msg_send![app, windowWithWindowNumber: window_number as isize];
            if window.is_null() {
                return None;
            }
            let view: *mut AnyObject = msg_send![window, contentView];
            let view = view.as_ref()?;
            let base = view.class();
            let name = CString::new(format!(
                "OpenNookFileDrop_{}",
                base.name().to_string_lossy()
            ))
            .ok()?;
            let subclass = if let Some(mut builder) = ClassBuilder::new(&name, base) {
                builder.add_method(
                    sel!(draggingEntered:),
                    updated as unsafe extern "C" fn(_, _, _) -> _,
                );
                builder.add_method(
                    sel!(draggingUpdated:),
                    updated as unsafe extern "C" fn(_, _, _) -> _,
                );
                builder.add_method(
                    sel!(draggingExited:),
                    exited as unsafe extern "C" fn(_, _, _),
                );
                builder.add_method(
                    sel!(prepareForDragOperation:),
                    prepare as unsafe extern "C" fn(_, _, _) -> _,
                );
                builder.add_method(
                    sel!(performDragOperation:),
                    perform as unsafe extern "C" fn(_, _, _) -> _,
                );
                builder.register()
            } else {
                AnyClass::get(&name)?
            };
            AnyObject::set_class(view, subclass);
            let kind: *mut AnyObject =
                msg_send![class!(NSString), stringWithUTF8String: c"public.file-url".as_ptr()];
            let types: *mut AnyObject = msg_send![class!(NSArray), arrayWithObject: kind];
            let _: () = msg_send![view, registerForDraggedTypes: types];
            let key = view as *const _ as usize;
            let pb: *mut AnyObject =
                msg_send![class!(NSPasteboard), pasteboardWithName: NSPasteboardNameDrag];
            let pasteboard_count: isize = msg_send![pb, changeCount];
            DESTINATIONS.with(|d| {
                d.borrow_mut().insert(
                    key,
                    Destination {
                        pasteboard_count,
                        ..Default::default()
                    },
                );
            });
            Some(key)
        }
    }

    pub fn region(key: usize, rect: [f32; 4], disabled: bool, approach_width: f32) {
        DESTINATIONS.with(|d| {
            if let Some(d) = d.borrow_mut().get_mut(&key) {
                d.rect = rect;
                d.approach_width = approach_width;
                d.disabled = disabled;
            }
        });
    }

    pub fn take(key: usize) -> Vec<Event> {
        // Expand below the camera before the pointer reaches the screen edge. AppKit
        // keeps the drag pasteboard after release, so only a fresh drag while held counts.
        unsafe {
            let pb: *mut AnyObject =
                msg_send![class!(NSPasteboard), pasteboardWithName: NSPasteboardNameDrag];
            let count: isize = msg_send![pb, changeCount];
            let buttons: usize = msg_send![class!(NSEvent), pressedMouseButtons];
            let point: Point = msg_send![class!(NSEvent), mouseLocation];
            let view = &*(key as *const AnyObject);
            let window: *mut AnyObject = msg_send![view, window];
            let frame: Rect = msg_send![window, frame];
            DESTINATIONS.with(|destinations| {
                let mut destinations = destinations.borrow_mut();
                let Some(d) = destinations.get_mut(&key) else {
                    return Vec::new();
                };
                if buttons & 1 == 0 {
                    d.dragging_files = false;
                }
                if count != d.pasteboard_count {
                    d.pasteboard_count = count;
                    d.dragging_files = buttons & 1 != 0 && !pasteboard_paths(pb).is_empty();
                }
                let [left, top, width, height] = d.rect;
                let x = (point.x - frame.origin.x) as f32;
                let y = (frame.origin.y + frame.size.h - point.y) as f32;
                let approach_left = left + (width - d.approach_width) / 2.0;
                let near = !d.disabled
                    && d.dragging_files
                    && x >= approach_left - 48.0
                    && x <= approach_left + d.approach_width + 48.0
                    && y >= top
                    && y <= top + height.max(240.0);
                if near != d.hovering {
                    d.hovering = near;
                    d.events.push(Event::Hover(near));
                }
                std::mem::take(&mut d.events)
            })
        }
    }
}

#[cfg(target_os = "macos")]
pub use mac::{install, region, take};
#[cfg(not(target_os = "macos"))]
pub fn install(_: u32) -> Option<usize> {
    None
}
#[cfg(not(target_os = "macos"))]
pub fn region(_: usize, _: [f32; 4], _: bool, _: f32) {}
#[cfg(not(target_os = "macos"))]
pub fn take(_: usize) -> Vec<Event> {
    Vec::new()
}
