# Android build

## One-time setup

1. **Android SDK and NDK** (e.g. installed with Android Studio, *SDK Manager*):
   Android SDK Platform 36, Build-Tools, Command-line Tools, Platform-Tools and
   NDK (Side by side).
2. **Environment variables** (user variables): `ANDROID_HOME` pointing to the
   SDK folder and `NDK_HOME` pointing to `<sdk>\ndk\<version>`.
3. **JDK 21**: Gradle 8.14 does not run on Java 25 (the JBR bundled with recent
   Android Studio). Install Temurin JDK 21 and point Gradle at it in
   `src-tauri/gen/android/gradle.properties`:
   ```
   org.gradle.java.home=C:/Program Files/Eclipse Adoptium/jdk-21...
   ```
4. **Rust targets**:
   ```
   rustup target add aarch64-linux-android armv7-linux-androideabi
   ```
5. **Windows Developer Mode** on (Settings → System → For developers): Tauri
   needs it to create symbolic links to the native libraries.

## Project settings already in place

- `minSdkVersion` 26 (`tauri.conf.json` and `app/build.gradle.kts`): needed by
  cpal's AAudio backend.
- `AndroidManifest.xml`: microphone, foreground service (microphone type),
  wake lock and notification permissions, and the `CountingService` that keeps
  counting with the screen off.
- `MainActivity.kt`: asks for the microphone and notification permissions and
  exposes `window.DaimokuAndroid` to the web page (counting service, keep
  screen on, save/share files).
- `proguard-rules.pro`: keeps the methods called from the web page in release
  builds.

## Signing (release builds)

1. Create an upload key once:
   ```
   keytool -genkey -v -keystore C:/Users/<you>/daimoku-upload.jks -keyalg RSA -keysize 2048 -validity 10000 -alias upload
   ```
   Keep the `.jks` file and its password safe: without them the app cannot be
   updated.
2. `src-tauri/gen/android/keystore.properties` (ignored by git):
   ```
   password=...
   keyAlias=upload
   storeFile=C:/Users/<you>/daimoku-upload.jks
   ```

## Icons

```
npm run tauri icon icon/icon-1024.png
```

## Build

Release APK for a phone (installs over the previous version, data kept):
```
npm run tauri android build -- --apk --target aarch64
```
The APK is in `src-tauri/gen/android/app/build/outputs/apk/universal/release/`.

Debug APK:
```
npm run tauri android build -- --apk --target aarch64 --debug
```

Play Store bundle:
```
npm run tauri android build -- --aab --target aarch64 --target armv7
```

Every upload needs a higher version (`version` in `tauri.conf.json`,
`package.json` and `src-tauri/Cargo.toml`).
