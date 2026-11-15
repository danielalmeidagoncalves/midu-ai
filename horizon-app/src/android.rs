//! Android-only platform calls.
//!
//! The live store stays app-private. A copy goes to the app's *external* files
//! directory, which is reachable over USB and from a file manager, falling back
//! to app-private storage when external storage is not mounted.

use std::path::PathBuf;

/// `Context.getExternalFilesDir(null)`, or `None` when it is unavailable.
pub fn external_files_dir() -> Option<PathBuf> {
    use jni::objects::{JObject, JString, JValue};

    let ctx = ndk_context::android_context();
    // Safety: ndk-context guarantees these pointers are valid once initialised.
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let null = JObject::null();
    let args = [JValue::Object(&null)];
    let dir = env
        .call_method(
            &context,
            "getExternalFilesDir",
            "(Ljava/lang/String;)Ljava/io/File;",
            &args,
        )
        .ok()?
        .l()
        .ok()?;
    if dir.is_null() {
        return None;
    }

    let path: JString = env
        .call_method(&dir, "getAbsolutePath", "()Ljava/lang/String;", &[])
        .ok()?
        .l()
        .ok()?
        .into();
    let text: String = env.get_string(&path).ok()?.into();
    Some(PathBuf::from(text))
}
