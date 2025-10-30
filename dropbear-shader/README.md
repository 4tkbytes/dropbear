# dropbear-shader

This crate is used for shaders created with the WESL language, such as rendering models and generating
mipmaps. It also provides templates for users to create their own shaders for their own 
projects with the WESL language. 

## What is WESL?

WESL (or WebGPU Extension Shader Language) is a shader language that is fully compatible with WGPU shaders, and
adds extension features such as import statements and rust-like #[cfg] functions. 