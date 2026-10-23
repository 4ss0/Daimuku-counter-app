# Android build

## One-time setup

1. **Android SDK and NDK** (e.g. installed with Android Studio, *SDK Manager*):
   Android SDK Platform 36, Build-Tools, Command-line Tools, Platform-Tools and
   NDK (Side by side).
2. **Environment variables** (user variables): `ANDROID_HOME` pointing to the
   SDK folder and `NDK_HOME` pointing to `<sdk>\ndk\<version>`.
3. **JDK 21**: Gradle 8.14 does not run on Java 25 (the JBR bundled with recent
   Android Studio). Install Temurin JDK 21 and point Gradle at it in your
   user Gradle settings, `C:/Users/<you>/.gradle/gradle.properties` (not in
   the project: the path is different on every computer):
   ```
   org.gradle.java.home=C:/Program Files/Eclipse Adoptium/jdk-21...
   ```
4. **Rust targets**:
   ```
   rustup target add aarch64-linux-android armv7-linux-androideabi
   ```
5. **Windows Developer Mode** on (Settings → System → For developers): Tauri
   needs it to create symbolic links to the native libraries.

## The Android project is in git

`src-tauri/gen/android` is part of the repository (since 0.6.0; before it was
only on the build PC). Files generated at every build (`tauri.properties`,
`tauri.build.gradle.kts`, the Rust libraries, ...) stay out of git through
its own `.gitignore`, and so do `keystore.properties` and `local.properties`.

**Moving from a local copy** (first pull after 0.5.3): git refuses to
overwrite the untracked folder. Rename it, pull, then copy back only
`keystore.properties`:
```
ren src-tauri\gen\android android-old
git pull
copy src-tauri\gen\android-old\keystore.properties src-tauri\gen\android\
```
(`ren` takes the new name only: the folder becomes `src-tauri\gen\android-old`.)
If `android-old` had changes of yours that are not below, compare its
`MainActivity.kt` and `AndroidManifest.xml` with the new ones.

## Project settings already in place

- `minSdkVersion` 26 (`tauri.conf.json` and `app/build.gradle.kts`): needed by
  cpal's AAudio backend.
- `AndroidManifest.xml`: microphone, foreground service (microphone type),
  wake lock and notification permissions, and the `CountingService` that keeps
  counting with the screen off. No INTERNET permission in release builds
  (`app/src/debug/AndroidManifest.xml` adds it to debug builds for
  `tauri android dev`).
- `MainActivity.kt`: asks for the microphone and notification permissions and
  exposes `window.DaimokuAndroid` to the web page (counting service, keep
  screen on, save/share files); see `src/lib/native.ts` for the other side.
- `CountingService.kt`: the foreground service and its notification, whose
  title (the count) the page updates at most once a second.
- `app/build.gradle.kts`: signs release builds with the key in
  `keystore.properties`, when the file is there.
- `proguard-rules.pro`: keeps the methods called from the web page in release
  builds.
- Icons: `res/mipmap-*`, made from `src-tauri/icons/icon.png`.

## Signing (release builds)

1. Create an upload key once:
   ```
   keytool -genkey -v -keystore C:/Users/<you>/daimoku-upload.jks -keyalg RSA -keysize 2048 -validity 10000 -alias upload
   ```
   Keep the `.jks` file and its password safe: without them the app cannot be
   updated.
2. `src-tauri/gen/android/keystore.properties` (ignored by git, read by
   `app/build.gradle.kts`):
   ```
   password=...
   keyAlias=upload
   storeFile=C:/Users/<you>/daimoku-upload.jks
   ```

## Icons

```
npm run tauri icon icon/icon-1024.png
```
This also rewrites `res/mipmap-*` in the Android project: commit them.

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
