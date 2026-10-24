# Add project specific ProGuard rules here.
# You can control the set of applied configuration files using the
# proguardFiles setting in build.gradle.
#
# For more details, see
#   http://developer.android.com/guide/developing/tools/proguard.html

# Methods called from the web page (window.DaimokuAndroid): keep them and
# their names in release builds.
-keepattributes JavascriptInterface
-keepclassmembers class com.daimuku.counter.MainActivity$Bridge {
   @android.webkit.JavascriptInterface <methods>;
}

# Uncomment this to preserve the line number information for
# debugging stack traces.
#-keepattributes SourceFile,LineNumberTable

# If you keep the line number information, uncomment this to
# hide the original source file name.
#-renamesourcefileattribute SourceFile

# the count read from the Rust engine (src-tauri/src/native.rs): JNI finds it by name
-keep class com.daimuku.counter.Native { native <methods>; }
