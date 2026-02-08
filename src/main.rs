#![allow(clippy::not_unsafe_ptr_arg_deref)]
use std::ffi::CStr;
use std::os::raw::c_char;

// Execute JavaScript using Emscripten functions
unsafe extern "C" {
    fn emscripten_run_script(script: *const c_char);
}

fn main() {
    // Method 1: Use println! (Emscripten automatically redirects to console.log)
    println!("Hello, world from Rust!");

    // Method 2: Execute JS directly to call console.log
    unsafe {
        let script = "console.log('Hello from Rust via emscripten_run_script!');\0";
        emscripten_run_script(script.as_ptr() as *const c_char);
    }
}

// Function called from JavaScript
// Receives form text, counts characters, and outputs to console
#[unsafe(no_mangle)]
pub extern "C" fn count_chars(text_ptr: *const c_char) {
    unsafe {
        // Convert C string to Rust string
        let c_str = CStr::from_ptr(text_ptr);
        let text = c_str.to_str().unwrap_or("");

        // Count characters
        let char_count = text.chars().count();

        // Output to console
        println!("Input text: {}", text);
        println!("Character count: {}", char_count);
    }
}
