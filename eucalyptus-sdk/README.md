# eucalyptus-sdk

This is the SDK that deals with plugins and its management, as well as user defined plugins. 

To use this crate, check out the related docs.rs (TODO ADD WEBSITE) to define your own plugin. 

Note: This SDK is only used in the eucalyptus-editor, and not in any games exported by it. If you wish to 
add your own dependencies to help other developers with scripting, you should create your own Kotlin Multiplatform 
library, or fork the dropbear-engine and add your own FFI interop functions. (Then recontribute it back to the 
repository.) 