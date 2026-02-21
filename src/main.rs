#![allow(clippy::not_unsafe_ptr_arg_deref)]
use std::ffi::CStr;
use std::os::raw::c_char;

use std::ptr::null_mut;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

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
#[cfg(target_arch = "wasm32")]
use mrubyedge::RObject;
#[cfg(target_arch = "wasm32")]
use mrubyedge::VM;
use mrubyedge::yamrb::helpers::{mrb_call_inspect, mrb_define_singleton_cmethod};

// JavaScript callback for system messages
#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    fn systemMessage(msg: *const c_char);

    fn getTimeSec() -> u64;
    fn getTimeNanosec() -> u32;
    fn getOffset() -> u32;
}

/// Wrapper function for systemMessage that accepts any type implementing Into<String>
#[cfg(target_arch = "wasm32")]
fn system_message(msg: impl Into<String>) {
    unsafe {
        let msg_string = msg.into();
        let c_msg = std::ffi::CString::new(msg_string)
            .unwrap_or_else(|_| std::ffi::CString::new("(conversion error)").unwrap());
        systemMessage(c_msg.as_ptr());
    }
}

#[cfg(target_arch = "wasm32")]
fn mrb_playground_time_source(
    vm: &mut VM,
    _args: &[Rc<RObject>],
) -> Result<Rc<RObject>, mrubyedge::Error> {
    unsafe {
        let sec = getTimeSec();
        let nanosec = getTimeNanosec();
        let offset = getOffset();
        let arr = vec![
            RObject::integer(sec as i64).to_refcount_assigned(),
            RObject::integer(nanosec as i64).to_refcount_assigned(),
            RObject::integer(offset as i64).to_refcount_assigned(),
        ];
        Ok(RObject::array(arr).to_refcount_assigned())
    }
}

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
    #[cfg(target_arch = "wasm32")]
    system_message("Environment initialized!");
}

/// Compiles Ruby source code into RITE bytecode and returns a pointer to it.
/// Returns null on compilation failure.
#[unsafe(no_mangle)]
pub extern "C" fn compile_ruby_script(text_ptr: *const c_char) -> *const u8 {
    unsafe {
        let c_str = CStr::from_ptr(text_ptr);
        let text = c_str.to_str().unwrap_or("");

        let mut context = MRubyCompiler2Context::new();

        let mrb = match context.compile(text) {
            Ok(bytecode) => bytecode,
            Err(e) => {
                eprintln!("Compilation error: {}", e);
                return std::ptr::null();
            }
        };

        mrb.leak().as_mut_ptr()
    }
}

/// Creates a VM on the fly and executes Ruby code to get the Ruby version
/// Returns a pointer to a C string like "mruby/edge - v3.3.0"
/// The returned string should be read with UTF8ToString() in JavaScript
#[unsafe(no_mangle)]
pub extern "C" fn show_ruby_version() -> *const c_char {
    let code = "\"#{RUBY_ENGINE} - v#{RUBY_VERSION}\"";

    let mut context = MRubyCompiler2Context::new();

    // Compile the Ruby script
    let mrb = match context.compile(code) {
        Ok(bytecode) => bytecode,
        Err(e) => {
            let error_msg = format!("! Compilation error: {}", e);
            return std::ffi::CString::new(error_msg)
                .unwrap_or_else(|_| std::ffi::CString::new("Compilation error").unwrap())
                .into_raw();
        }
    };

    // Load and execute the bytecode
    let mut rite = match mrubyedge::rite::load(&mrb) {
        Ok(r) => r,
        Err(e) => {
            let error_msg = format!("! Failed to load bytecode: {:?}", e);
            return std::ffi::CString::new(error_msg)
                .unwrap_or_else(|_| std::ffi::CString::new("Bytecode load error").unwrap())
                .into_raw();
        }
    };

    let mut vm = mrubyedge::yamrb::vm::VM::open(&mut rite);

    // Execute the script and handle exceptions
    let result = match vm.run() {
        Ok(r) => r,
        Err(e) => {
            let error_msg = format!("! Runtime error: {:?}", e);
            return std::ffi::CString::new(error_msg)
                .unwrap_or_else(|_| std::ffi::CString::new("! Runtime error").unwrap())
                .into_raw();
        }
    };

    // Convert result to string
    let result_string: String = match result.as_ref().try_into() {
        Ok(s) => s,
        Err(e) => {
            format!("! Type Mismatch {}", e)
        }
    };

    // Convert to C string and return pointer
    std::ffi::CString::new(result_string)
        .unwrap_or_else(|_| std::ffi::CString::new("! conversion error").unwrap())
        .into_raw()
}

// Function called from JavaScript
// Receives Ruby script and optional random seed, executes it, and outputs the result
#[unsafe(no_mangle)]
pub extern "C" fn load_ruby_script(text_ptr: *const c_char, seed: u32) {
    unsafe {
        // Convert C string to Rust string
        let c_str = CStr::from_ptr(text_ptr);
        let text = c_str.to_str().unwrap_or("");

        let mut context = MRubyCompiler2Context::new();

        // Prepend Random.srand if seed is provided
        let code_with_seed = if seed != 0 {
            format!("Random.srand({}); {}", seed, text)
        } else {
            text.to_string()
        };

        // Compile the Ruby script
        let mrb = match context.compile(&code_with_seed) {
            Ok(bytecode) => bytecode,
            Err(e) => {
                #[cfg(target_arch = "wasm32")]
                {
                    system_message(format!("Compilation error: {}", e));
                    system_message("This may be a platform bug");
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    eprintln!("Compilation error: {}", e);
                    eprintln!("This may be a platform bug");
                }
                return;
            }
        };

        // Load and execute the bytecode
        let mut rite = match mrubyedge::rite::load(&mrb) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to load bytecode: {:?}", e);
                return;
            }
        };

        let mut vm = mrubyedge::yamrb::vm::VM::open(&mut rite);
        mrubyedge_serde_json::init_json(&mut vm);
        mrubyedge_math::init_math(&mut vm);
        mrubyedge_time::init_time(&mut vm);
        #[cfg(target_arch = "wasm32")]
        {
            let time_class_obj = vm
                .get_const_by_name("Time")
                .expect("Failed to define Time class");
            mrb_define_singleton_cmethod(
                &mut vm,
                time_class_obj,
                "__source",
                Box::new(mrb_playground_time_source),
            );
        }

        // Execute the script and handle exceptions
        let result = match vm.run() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Runtime error: {:?}", e);
                return;
            }
        };

        // Convert result to inspect string
        let result_as_inspect: String = match mrb_call_inspect(&mut vm, result) {
            Ok(inspect_value) => match inspect_value.as_ref().try_into() {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("Failed to convert inspect result to string");
                    return;
                }
            },
            Err(e) => {
                eprintln!("Failed to call inspect: {:?}", e);
                return;
            }
        };

        // Output result using JavaScript callback
        #[cfg(target_arch = "wasm32")]
        system_message(format!("Result: {}", result_as_inspect));
        #[cfg(not(target_arch = "wasm32"))]
        println!("Result: {}", result_as_inspect);
    }
}
