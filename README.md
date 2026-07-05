# bevy

A minimal bevy application rendering the projects logo.

To run this on desktop, do `cargo run`.
To run on a connected mobile device, use `cargo android run` or `cargo apple run`.
Open the mobile projects in Android Studio or Xcode with `cargo android open` or `cargo apple open` respectively.


cargo android build
cd gen/android  
./gradlew assembleDebug 
cd ../.. 
adb install -r gen/android/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk 
adb shell am start -n com.wbt.thesilentreach/com.google.androidgamesdk.GameActivity 
sleep 3 
adb logcat -d | grep -iE "bevy|rust|sqlite|persistence|CannotOpen|error|warn|GameActivity" | tail -n 80
