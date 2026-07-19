# JNI entry points are looked up by class and method name from Rust.
# Keep the bridge intact in release builds so R8 cannot rename or remove it.
-keep class __PACKAGE__.PcmAudioTrackBridge { *; }
-keepclassmembers class __PACKAGE__.PcmAudioTrackBridge {
  public static *;
}

-keep class __PACKAGE__.SenseVoiceBridge { *; }
-keep class __PACKAGE__.ZipformerCtcBridge { *; }
-keep class com.k2fsa.sherpa.onnx.** { *; }
