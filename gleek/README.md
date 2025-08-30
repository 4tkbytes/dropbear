# gleek
This crate contains proc-macros that convert types and functions into modules for the Gleam language to 
reference as **stubs** into your own Gleam projects and allow type safety + better debugging with the LSP. 

## Requirements
Gleam can only be compiled to JavaScript to be used with rust, so it is not compatible with a normal Erlang compilation. 