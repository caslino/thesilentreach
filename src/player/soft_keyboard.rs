use bevy::prelude::*;

#[cfg(target_os = "android")]
pub fn show_keyboard() {
    use jni::JavaVM;
    use jni::objects::JObject;

    let res = (|| -> Result<(), Box<dyn std::error::Error>> {
        let ctx = ndk_context::android_context();
        let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }?;
        let mut env = vm.attach_current_thread()?;

        let context = unsafe { JObject::from_raw(ctx.context() as jni::sys::jobject) };
        let window = env.call_method(&context, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
        let decor_view = env.call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?.l()?;

        let imm_string = env.new_string("input_method")?;
        let imm = env.call_method(&context, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[(&imm_string).into()])?.l()?;

        env.call_method(&imm, "showSoftInput", "(Landroid/view/View;I)Z", &[(&decor_view).into(), 0.into()])?;
        Ok(())
    })();
    if let Err(e) = res {
        error!("Failed to show soft keyboard via JNI: {:?}", e);
    }
}

#[cfg(target_os = "android")]
pub fn hide_keyboard() {
    use jni::JavaVM;
    use jni::objects::JObject;

    let res = (|| -> Result<(), Box<dyn std::error::Error>> {
        let ctx = ndk_context::android_context();
        let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }?;
        let mut env = vm.attach_current_thread()?;

        let context = unsafe { JObject::from_raw(ctx.context() as jni::sys::jobject) };
        let window = env.call_method(&context, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
        let decor_view = env.call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?.l()?;

        let imm_string = env.new_string("input_method")?;
        let imm = env.call_method(&context, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[(&imm_string).into()])?.l()?;

        let window_token = env.call_method(&decor_view, "getWindowToken", "()Landroid/os/IBinder;", &[])?.l()?;

        env.call_method(&imm, "hideSoftInputFromWindow", "(Landroid/os/IBinder;I)Z", &[(&window_token).into(), 0.into()])?;
        Ok(())
    })();
    if let Err(e) = res {
        error!("Failed to hide soft keyboard via JNI: {:?}", e);
    }
}

#[cfg(not(target_os = "android"))]
pub fn show_keyboard() {}

#[cfg(not(target_os = "android"))]
pub fn hide_keyboard() {}
