#![allow(clippy::not_unsafe_ptr_arg_deref)]
use std::ffi::CStr;
use std::os::raw::c_char;

// Execute JavaScript using Emscripten functions
unsafe extern "C" {
    fn emscripten_run_script(script: *const c_char);
}

use std::ptr::null_mut;

mod bindings {
    #![allow(nonstandard_style)]
    #![allow(unused)]
    #![allow(unnecessary_transmutes)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

    #[cfg(target_arch = "wasm32")]
    unsafe extern "C" {
        pub fn mrc_ccontext_new(mrb: *mut ::std::os::raw::c_void) -> *mut mrc_ccontext;
        pub fn mrc_ccontext_free(c: *mut mrc_ccontext);
        pub fn mrc_load_string_cxt(
            c: *mut mrc_ccontext,
            source: *mut *const u8,
            length: usize,
        ) -> *mut mrc_irep;
        pub fn mrc_dump_irep(
            c: *mut mrc_ccontext,
            irep: *const mrc_irep,
            flags: u8,
            bin: *mut *mut u8,
            bin_size: *mut usize,
        ) -> ::std::os::raw::c_int;
        pub fn mrc_irep_free(c: *mut mrc_ccontext, irep: *mut mrc_irep);
    }
}

use bindings::{
    MRC_DUMP_OK, mrc_ccontext, mrc_ccontext_free, mrc_ccontext_new, mrc_dump_irep, mrc_irep,
    mrc_irep_free, mrc_load_string_cxt,
};
use mrubyedge::yamrb::helpers::mrb_call_inspect;

#[derive(Debug)]
pub struct MRubyCompiler2Error {
    details: String,
}

impl MRubyCompiler2Error {
    fn new(msg: &str) -> MRubyCompiler2Error {
        MRubyCompiler2Error {
            details: msg.to_string(),
        }
    }

    #[allow(unused)]
    fn from_error<E: std::error::Error>(msg: &str, err: E) -> MRubyCompiler2Error {
        MRubyCompiler2Error {
            details: format!("{}: {}", msg, err),
        }
    }
}

impl std::fmt::Display for MRubyCompiler2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl std::error::Error for MRubyCompiler2Error {}

pub struct MRubyCompiler2Context {
    c: *mut mrc_ccontext,
}

impl MRubyCompiler2Context {
    #[allow(clippy::new_without_default)]
    /// Creates a new MRubyCompiler2Context
    pub fn new() -> Self {
        unsafe {
            let ccontext = mrc_ccontext_new(null_mut());
            MRubyCompiler2Context { c: ccontext }
        }
    }

    /// Compiles the given mruby code into mruby bytecode binary
    /// Returns the bytecode as a `Vec<u8>`
    pub fn compile(&mut self, code: &str) -> Result<Vec<u8>, MRubyCompiler2Error> {
        unsafe {
            let c_code = std::ffi::CString::new(code)
                .map_err(|_| MRubyCompiler2Error::new("Code includes null bytes"))?;
            let mut ptr = c_code.as_ptr() as *const u8;
            let irep =
                mrc_load_string_cxt(self.c, &mut ptr as *mut *const u8, c_code.as_bytes().len());

            if irep.is_null() {
                return Err(MRubyCompiler2Error::new("Failed to compile code"));
            }

            // Set dummy capacity, deduced from code length
            // And leak for safety rather than memory efficiency
            let bin: &'static mut [u8] = Vec::with_capacity(code.len() * 2).leak();
            let bin_ptr = bin.as_mut_ptr();
            let mut bin_size: usize = 0;

            let result = mrc_dump_irep(
                self.c,
                irep as *mut mrc_irep,
                0,
                &bin_ptr as *const *mut u8 as *mut *mut u8,
                &mut bin_size as *mut usize,
            );
            mrc_irep_free(self.c, irep as *mut mrc_irep);
            if result as u32 != MRC_DUMP_OK {
                return Err(MRubyCompiler2Error::new("Failed to dump irep binary"));
            }

            let newvec = Vec::from_raw_parts(bin_ptr, bin_size, bin_size);
            Ok(newvec)
        }
    }
}

impl Drop for MRubyCompiler2Context {
    fn drop(&mut self) {
        unsafe {
            mrc_ccontext_free(self.c);
        }
    }
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

        let mut context = MRubyCompiler2Context::new();
        let mrb = context.compile(text).unwrap();

        let mut rite = mrubyedge::rite::load(&mrb).unwrap();
        let mut vm = mrubyedge::yamrb::vm::VM::open(&mut rite);
        mruby_serde_json::init_json(&mut vm);
        let result = vm.run().unwrap();
        let result_as_inspect: String = mrb_call_inspect(&mut vm, result)
            .unwrap()
            .as_ref()
            .try_into()
            .unwrap();

        // Output to console
        println!("Input text: {}", text);
        println!("Result: {}", result_as_inspect);
    }
}
