# JNI entry points are looked up by class and method name from Rust.
# Keep the bridge intact in release builds so R8 cannot rename or remove it.
-keep class com.lettuceai.app.PcmAudioTrackBridge { *; }
-keepclassmembers class com.lettuceai.app.PcmAudioTrackBridge {
  public static *;
}

-keep class com.lettuceai.app.SenseVoiceBridge { *; }
-keep class com.lettuceai.app.ZipformerCtcBridge { *; }
-keep class com.k2fsa.sherpa.onnx.** { *; }
