//! C ABI for `aa-core`.
//!
//! Compiled as `staticlib` for iOS (linked into the native Swift shell's
//! AaEngine.xcframework) and `cdylib` for Android (loaded as `libaa_ffi.so`
//! by the native Kotlin shell's live wallpaper service).
//!
//! Frame buffer layout — 8 bytes per cell:
//!   [0–3]  Unicode scalar as u32 LE
//!   [4]    R (0 when has_color == 0)
//!   [5]    G
//!   [6]    B
//!   [7]    has_color: 1 = use RGB above, 0 = use theme colour in shell

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double};

use aa_core::frame::Frame;
use aa_core::scene::Scene;
use aa_core::scenes::{self, BUILTIN_IDS};
use aa_core::theme::Theme;

const BYTES_PER_CELL: usize = 8;

/// Encode `frame` into `out` using the shared 8-bytes-per-cell wire layout
/// (see the module doc comment). `out` is resized in place and reused across
/// calls by the caller to avoid per-frame allocation.
fn encode_frame(frame: &Frame, out: &mut Vec<u8>) {
    let num_cells = frame.width * frame.height;
    out.resize(num_cells * BYTES_PER_CELL, 0);

    for (i, cell) in frame.cells.iter().enumerate() {
        let off = i * BYTES_PER_CELL;
        out[off..off + 4].copy_from_slice(&(cell.ch as u32).to_le_bytes());
        let (rgb, has_color) = match cell.color {
            Some(c) => ([c.r, c.g, c.b], 1u8),
            None => ([0, 0, 0], 0u8),
        };
        out[off + 4..off + 7].copy_from_slice(&rgb);
        out[off + 7] = has_color;
    }
}

pub struct AaEngine {
    scene: Box<dyn Scene + Send>,
    frame_buf: Vec<u8>,
}

impl AaEngine {
    fn new(scene_id: &str) -> Option<Self> {
        scenes::make(scene_id).map(|scene| AaEngine {
            scene,
            frame_buf: Vec::new(),
        })
    }
}

unsafe fn str_from_ptr<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok()
}

// ── Public C API ─────────────────────────────────────────────────────────────

/// Construct an engine running the named scene.
/// Valid scene ids: "donut", "helix", "matrix", "pipes", "life".
/// Returns NULL for unknown ids.
#[no_mangle]
pub extern "C" fn aa_engine_create(scene_id: *const c_char) -> *mut AaEngine {
    let id = unsafe {
        match str_from_ptr(scene_id) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        }
    };
    match AaEngine::new(id) {
        Some(engine) => Box::into_raw(Box::new(engine)),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn aa_engine_destroy(engine: *mut AaEngine) {
    if !engine.is_null() {
        unsafe { drop(Box::from_raw(engine)) };
    }
}

#[no_mangle]
pub extern "C" fn aa_engine_set_grid(engine: *mut AaEngine, width: u32, height: u32) {
    if engine.is_null() {
        return;
    }
    let e = unsafe { &mut *engine };
    e.scene.set_grid(width as usize, height as usize);
}

/// Set the colour theme by name ("Hacker", "Amber", "Ice", "Ghost").
/// Unknown names are silently ignored.
#[no_mangle]
pub extern "C" fn aa_engine_set_theme(engine: *mut AaEngine, theme_name: *const c_char) {
    if engine.is_null() {
        return;
    }
    let name = unsafe {
        match str_from_ptr(theme_name) {
            Some(s) => s,
            None => return,
        }
    };
    let e = unsafe { &mut *engine };
    if let Some(theme) = Theme::by_name(name) {
        e.scene.apply_base_color(theme.text);
    }
}

/// Forward a scene-specific setting (id + numeric value).
#[no_mangle]
pub extern "C" fn aa_engine_apply_setting(
    engine: *mut AaEngine,
    setting_id: *const c_char,
    value: c_double,
) {
    if engine.is_null() {
        return;
    }
    let id = unsafe {
        match str_from_ptr(setting_id) {
            Some(s) => s,
            None => return,
        }
    };
    let e = unsafe { &mut *engine };
    e.scene.apply_setting(id, value);
}

/// Render the next frame at animation time `t` (seconds).
///
/// Returns a pointer to a flat byte buffer owned by the engine — valid until
/// the next call to `aa_engine_next_frame` or `aa_engine_destroy`.
/// Returns NULL on error.
///
/// Buffer layout: width * height cells, 8 bytes each:
///   `[0–3]` char as uint32_t little-endian (Unicode scalar value)
///   `[4]`   red   (0 when has_color == 0)
///   `[5]`   green
///   `[6]`   blue
///   `[7]`   has_color: 1 = use rgb above, 0 = use the active theme colour
#[no_mangle]
pub extern "C" fn aa_engine_next_frame(
    engine: *mut AaEngine,
    t: c_double,
    out_width: *mut u32,
    out_height: *mut u32,
) -> *const u8 {
    if engine.is_null() {
        return std::ptr::null();
    }
    let e = unsafe { &mut *engine };
    let frame = e.scene.frame(t);

    if !out_width.is_null() {
        unsafe { *out_width = frame.width as u32 };
    }
    if !out_height.is_null() {
        unsafe { *out_height = frame.height as u32 };
    }

    // Encode frame into the internal buffer (reused across calls).
    encode_frame(&frame, &mut e.frame_buf);

    e.frame_buf.as_ptr()
}

/// Return a null-terminated array of built-in scene id strings.
/// `*out_count` is set to the number of ids (excluding the null terminator).
/// Free the result with `aa_scene_names_free(names, count)`.
#[no_mangle]
pub extern "C" fn aa_scene_names(out_count: *mut u32) -> *mut *mut c_char {
    if !out_count.is_null() {
        unsafe { *out_count = BUILTIN_IDS.len() as u32 };
    }
    let mut ptrs: Vec<*mut c_char> = BUILTIN_IDS
        .iter()
        .map(|s| CString::new(*s).unwrap().into_raw())
        .collect();
    ptrs.push(std::ptr::null_mut());
    let ptr = ptrs.as_mut_ptr();
    std::mem::forget(ptrs);
    ptr
}

/// Free a names array returned by `aa_scene_names`.
#[no_mangle]
pub extern "C" fn aa_scene_names_free(names: *mut *mut c_char, count: u32) {
    if names.is_null() {
        return;
    }
    unsafe {
        for i in 0..count as usize {
            let p = *names.add(i);
            if !p.is_null() {
                drop(CString::from_raw(p));
            }
        }
        // Reconstruct the Vec (length = count + 1 for the null terminator) to free it.
        let cap = count as usize + 1;
        drop(Vec::from_raw_parts(names, cap, cap));
    }
}

// ── Android JNI bridge ───────────────────────────────────────────────────────
//
// Class: com.builder106.asciiarcade.engine.AaEngineNative — must be a
// top-level Kotlin `object`, not a companion object (a companion-scoped
// `external fun` mangles to a `$Companion`-suffixed symbol that would not
// match these names).
// The handle passed to every JNI function is the `AaEngine*` cast to `jlong`.

#[cfg(target_os = "android")]
mod android {
    use super::*;
    use jni::errors::LogErrorAndDefault;
    use jni::jni_str;
    use jni::objects::{JClass, JString};
    use jni::sys::{jbyteArray, jdouble, jint, jlong, jobjectArray};
    use jni::{Env, EnvUnowned};

    // jni 0.22 split the old `JNIEnv` into `EnvUnowned` (the FFI-safe type
    // received as a native-method parameter) and `Env` (the full JNI API,
    // only reachable inside an `EnvUnowned::with_env` closure). `with_env`
    // wraps the closure in `catch_unwind` and `.resolve::<LogErrorAndDefault>()`
    // maps both errors and panics to a logged message plus a default return
    // value (0/null) — the same "fail quietly, return a sentinel" behavior
    // this bridge already wanted, now provided by the crate instead of a
    // hand-rolled `.unwrap_or(null)`.
    fn jstring_to_string(env: &Env<'_>, s: &JString<'_>) -> jni::errors::Result<String> {
        Ok(s.mutf8_chars(env)?.into())
    }

    #[no_mangle]
    pub extern "system" fn Java_com_builder106_asciiarcade_engine_AaEngineNative_nativeCreate<
        'local,
    >(
        mut unowned_env: EnvUnowned<'local>,
        _class: JClass<'local>,
        scene_name: JString<'local>,
    ) -> jlong {
        unowned_env
            .with_env(|env| -> jni::errors::Result<jlong> {
                let name = jstring_to_string(env, &scene_name)?;
                Ok(match AaEngine::new(&name) {
                    Some(engine) => Box::into_raw(Box::new(engine)) as jlong,
                    None => 0,
                })
            })
            .resolve::<LogErrorAndDefault>()
    }

    // No JNI calls in the body, so this never needs to upgrade EnvUnowned to
    // Env via with_env().
    #[no_mangle]
    pub extern "system" fn Java_com_builder106_asciiarcade_engine_AaEngineNative_nativeDestroy<
        'local,
    >(
        _env: EnvUnowned<'local>,
        _class: JClass<'local>,
        handle: jlong,
    ) {
        if handle != 0 {
            unsafe { drop(Box::from_raw(handle as *mut AaEngine)) };
        }
    }

    #[no_mangle]
    pub extern "system" fn Java_com_builder106_asciiarcade_engine_AaEngineNative_nativeSetGrid<
        'local,
    >(
        _env: EnvUnowned<'local>,
        _class: JClass<'local>,
        handle: jlong,
        width: jint,
        height: jint,
    ) {
        if handle == 0 {
            return;
        }
        let e = unsafe { &mut *(handle as *mut AaEngine) };
        e.scene.set_grid(width as usize, height as usize);
    }

    #[no_mangle]
    pub extern "system" fn Java_com_builder106_asciiarcade_engine_AaEngineNative_nativeSetTheme<
        'local,
    >(
        mut unowned_env: EnvUnowned<'local>,
        _class: JClass<'local>,
        handle: jlong,
        theme_name: JString<'local>,
    ) {
        unowned_env
            .with_env(|env| -> jni::errors::Result<()> {
                if handle == 0 {
                    return Ok(());
                }
                let name = jstring_to_string(env, &theme_name)?;
                let e = unsafe { &mut *(handle as *mut AaEngine) };
                if let Some(theme) = Theme::by_name(&name) {
                    e.scene.apply_base_color(theme.text);
                }
                Ok(())
            })
            .resolve::<LogErrorAndDefault>()
    }

    #[no_mangle]
    pub extern "system" fn Java_com_builder106_asciiarcade_engine_AaEngineNative_nativeApplySetting<
        'local,
    >(
        mut unowned_env: EnvUnowned<'local>,
        _class: JClass<'local>,
        handle: jlong,
        setting_id: JString<'local>,
        value: jdouble,
    ) {
        unowned_env
            .with_env(|env| -> jni::errors::Result<()> {
                if handle == 0 {
                    return Ok(());
                }
                let id = jstring_to_string(env, &setting_id)?;
                let e = unsafe { &mut *(handle as *mut AaEngine) };
                e.scene.apply_setting(&id, value);
                Ok(())
            })
            .resolve::<LogErrorAndDefault>()
    }

    #[no_mangle]
    pub extern "system" fn Java_com_builder106_asciiarcade_engine_AaEngineNative_nativeNextFrame<
        'local,
    >(
        mut unowned_env: EnvUnowned<'local>,
        _class: JClass<'local>,
        handle: jlong,
        t: jdouble,
    ) -> jbyteArray {
        unowned_env
            .with_env(|env| -> jni::errors::Result<jbyteArray> {
                if handle == 0 {
                    return Ok(std::ptr::null_mut());
                }
                let e = unsafe { &mut *(handle as *mut AaEngine) };
                let frame = e.scene.frame(t);
                encode_frame(&frame, &mut e.frame_buf);

                // JNI byte[] is signed, but we pass raw bytes — the Kotlin
                // side reads them as unsigned via `and(0xFF)`.
                let byte_slice: &[i8] = unsafe {
                    std::slice::from_raw_parts(e.frame_buf.as_ptr() as *const i8, e.frame_buf.len())
                };
                let arr = env.new_byte_array(e.frame_buf.len())?;
                arr.set_region(env, 0, byte_slice)?;
                Ok(arr.into_raw())
            })
            .resolve::<LogErrorAndDefault>()
    }

    #[no_mangle]
    pub extern "system" fn Java_com_builder106_asciiarcade_engine_AaEngineNative_nativeSceneNames<
        'local,
    >(
        mut unowned_env: EnvUnowned<'local>,
        _class: JClass<'local>,
    ) -> jobjectArray {
        unowned_env
            .with_env(|env| -> jni::errors::Result<jobjectArray> {
                let string_class = env.find_class(jni_str!("java/lang/String"))?;
                let empty = env.new_string("")?;
                let arr = env.new_object_array(BUILTIN_IDS.len() as i32, &string_class, &empty)?;
                for (i, id) in BUILTIN_IDS.iter().enumerate() {
                    let s = env.new_string(*id)?;
                    arr.set_element(env, i, &s)?;
                }
                Ok(arr.into_raw())
            })
            .resolve::<LogErrorAndDefault>()
    }
}
