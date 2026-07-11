#[cfg(target_os = "android")]
mod android {
    use std::sync::mpsc;

    use jni::objects::{JClass, JValue};
    use tauri::{AppHandle, Manager};

    pub struct Player {
        app: AppHandle,
    }

    impl Player {
        pub fn start(app: &AppHandle, sample_rate: u32) -> Result<Option<Self>, String> {
            let ok = call_bridge(app, move |env, activity, class| {
                let result = env.call_static_method(
                    class,
                    "start",
                    "(Landroid/content/Context;III)Z",
                    &[
                        JValue::Object(activity),
                        JValue::Int(sample_rate as i32),
                        JValue::Int(1),
                        JValue::Int(16),
                    ],
                )?;
                Ok(result.z()?)
            })?;
            if ok {
                Ok(Some(Self { app: app.clone() }))
            } else {
                Ok(None)
            }
        }

        pub fn write(&self, bytes: &[u8]) -> Result<(), String> {
            let bytes = bytes.to_vec();
            let byte_len = bytes.len();
            let written = call_bridge(&self.app, move |env, _activity, class| {
                let data = env.byte_array_from_slice(&bytes)?;
                let result =
                    env.call_static_method(class, "write", "([B)I", &[JValue::Object(&data)])?;
                Ok(result.i()?)
            })?;
            if written == byte_len as i32 {
                Ok(())
            } else if written < 0 {
                Err(format!("Android AudioTrack write failed: {}", written))
            } else {
                Err(format!(
                    "Android AudioTrack wrote {} of {} bytes",
                    written, byte_len
                ))
            }
        }

        pub fn stop(&self) {
            let _ = call_bridge(&self.app, |env, _activity, class| {
                env.call_static_method(class, "stop", "()V", &[])?;
                Ok(())
            });
        }
    }

    fn call_bridge<T, F>(app: &AppHandle, callback: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce(
                &mut jni::JNIEnv<'_>,
                &jni::objects::JObject<'_>,
                JClass<'_>,
            ) -> Result<T, jni::errors::Error>
            + Send
            + 'static,
    {
        let window = app
            .get_webview_window("main")
            .ok_or_else(|| "Android PCM bridge requires the main WebView".to_string())?;
        let (tx, rx) = mpsc::channel();
        let bridge_name = option_env!("PCM_AUDIO_TRACK_ANDROID_BRIDGE_CLASS")
            .unwrap_or("com.lettuceai.app.PcmAudioTrackBridge")
            .to_string();
        window
            .with_webview(move |webview| {
                webview.jni_handle().exec(move |env, activity, _webview| {
                    let result = (|| -> Result<T, jni::errors::Error> {
                        let class_loader = env
                            .call_method(
                                activity,
                                "getClassLoader",
                                "()Ljava/lang/ClassLoader;",
                                &[],
                            )?
                            .l()?;
                        let class_name = env.new_string(bridge_name)?;
                        let class_obj = env
                            .call_method(
                                &class_loader,
                                "loadClass",
                                "(Ljava/lang/String;)Ljava/lang/Class;",
                                &[JValue::Object(&class_name)],
                            )?
                            .l()?;
                        let class: JClass = class_obj.into();
                        callback(env, activity, class)
                    })();
                    if env.exception_check().unwrap_or(false) {
                        // JNI otherwise collapses the useful Android stack trace into
                        // "Java exception was thrown" before it reaches Rust.
                        let _ = env.exception_describe();
                        let _ = env.exception_clear();
                    }
                    let _ = tx.send(result.map_err(|error| error.to_string()));
                });
            })
            .map_err(|error| error.to_string())?;
        rx.recv().map_err(|error| error.to_string())?
    }
}

#[cfg(target_os = "android")]
pub use android::Player;

#[cfg(not(target_os = "android"))]
pub struct Player;

#[cfg(not(target_os = "android"))]
impl Player {
    pub fn start(_app: &tauri::AppHandle, _sample_rate: u32) -> Result<Option<Self>, String> {
        Ok(None)
    }

    pub fn write(&self, _bytes: &[u8]) -> Result<(), String> {
        Ok(())
    }

    pub fn stop(&self) {}
}
