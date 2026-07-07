# Add project specific ProGuard rules here.
# By default, the flags in this file are appended to flags specified
# in /sdk/tools/proguard/proguard-android.txt

# Keep Kotlinx Serialization
-keepattributes *Annotation*, InnerClasses
-dontnote kotlinx.serialization.AnnotationsKt
-keepclassmembers class kotlinx.serialization.json.** {
    *** Companion;
}
-keepclasseswithmembers class kotlinx.serialization.json.** {
    kotlinx.serialization.KSerializer serializer(...);
}
-keep,includedescriptorclasses class com.simtropolis.sim.**$$serializer { *; }
-keepclassmembers class com.simtropolis.sim.** {
    *** Companion;
}
-keepclasseswithmembers class com.simtropolis.sim.** {
    kotlinx.serialization.KSerializer serializer(...);
}
