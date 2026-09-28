# Prima build Android (emulatore su Windows 11)

## 1. Una volta sola: strumenti

1. Installa **Android Studio**. Da *More Actions → SDK Manager*:
   - *SDK Platforms*: Android 14 (API 34) o più recente
   - *SDK Tools*: **Android SDK Build-Tools**, **Android SDK Command-line Tools**,
     **Android SDK Platform-Tools**, **NDK (Side by side)**
2. Variabili d'ambiente (Impostazioni → Sistema → Informazioni → Impostazioni di sistema
   avanzate → Variabili d'ambiente), come *variabili utente*:
   - `JAVA_HOME` = `C:\Program Files\Android\Android Studio\jbr`
   - `ANDROID_HOME` = `%LOCALAPPDATA%\Android\Sdk`
   - `NDK_HOME` = `%LOCALAPPDATA%\Android\Sdk\ndk\<versione>` (la cartella che c'è dentro `ndk`)
3. Attiva la **Modalità sviluppatore** di Windows (Impostazioni → Sistema → Per sviluppatori):
   Tauri ne ha bisogno per creare i collegamenti alle librerie.
4. Nuovo terminale, poi:
   ```
   rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
   ```

## 2. Progetto

1. In `src-tauri/Cargo.toml` usa **cpal 0.16** (la 0.15 su Android richiede librerie C++):
   ```toml
   cpal = "0.16"
   ```
2. Genera il progetto Android:
   ```
   npm run tauri android init
   ```
3. **Permesso microfono** — in `src-tauri/gen/android/app/src/main/AndroidManifest.xml`,
   sotto le altre righe `<uses-permission ...>`, aggiungi:
   ```xml
   <uses-permission android:name="android.permission.RECORD_AUDIO" />
   ```
4. **MainActivity** — apri `src-tauri/gen/android/app/src/main/java/.../MainActivity.kt`,
   lascia la prima riga `package ...` e sostituisci il resto con `android/MainActivity.kt`
   di questo zip.

## 3. Emulatore

1. Android Studio → *Device Manager* → crea un dispositivo (es. Pixel 7, immagine
   **x86_64**, API 34) e avvialo.
2. Microfono: nella barra dell'emulatore **⋯ (Extended controls) → Microphone** →
   attiva **"Virtual microphone uses host audio input"**. Va riattivato a ogni avvio
   dell'emulatore.

## 4. Build e installazione

APK di debug per l'emulatore (non serve il server di sviluppo):
```
npm run tauri -- android build --apk --target x86_64 --debug
```
L'APK è in
`src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`:
trascinalo sulla finestra dell'emulatore per installarlo.

In alternativa, con l'emulatore acceso: `npm run tauri android dev`
(ricarica automatica; richiede che `vite.config` usi `TAURI_DEV_HOST`, come nel
template di create-tauri-app).

Al primo avvio l'app chiede il permesso del microfono: consenti.
