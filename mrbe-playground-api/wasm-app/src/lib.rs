#![allow(static_mut_refs)]
extern crate mrubyedge;
extern crate uzumibi_gem;

use std::{mem::MaybeUninit, rc::Rc};

use mrubyedge::{
    rite::rite,
    yamrb::{
        helpers::{mrb_define_class_cmethod, mrb_define_cmethod, mrb_funcall},
        value::{RModule, RObject, RValue},
        vm::VM,
    },
};

static MRB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/app.mrb"));

static mut MRUBY_VM: MaybeUninit<VM> = MaybeUninit::uninit();
static mut MRUBY_VM_LOADED: bool = false;

static mut ERROR_BUF: [u8; 4096] = [0; 4096];

fn set_error_to_buf(message: impl AsRef<str>) -> *const u8 {
    unsafe {
        let bytes = message.as_ref().as_bytes();
        let len = bytes.len().min(ERROR_BUF.len() - 1);
        ERROR_BUF[..len].copy_from_slice(&bytes[..len]);
        ERROR_BUF[len] = 0;
        ERROR_BUF.as_ptr()
    }
}

unsafe extern "C" {
    unsafe fn debug_console_log(ptr: *const u8, len: usize);
    unsafe fn do_storage_get(
        key_ptr: *const u8,
        key_size: usize,
        result_ptr: *mut u8,
        result_max_size: usize,
    ) -> i32;
    unsafe fn do_storage_set(
        key_ptr: *const u8,
        key_size: usize,
        value_ptr: *const u8,
        value_size: usize,
    ) -> i32;
}

fn debug_console_log_internal(message: &str) {
    unsafe {
        debug_console_log(message.as_ptr(), message.len());
    }
}

/// Get a value from Durable Object storage cache
/// Returns Ok(Some(value)) if found, Ok(None) if not found
/// The buffer size is limited to 64KB
fn storage_get(key: &str) -> Result<Option<String>, String> {
    const BUFFER_SIZE: usize = 65536;
    let mut buffer = vec![0u8; BUFFER_SIZE];

    unsafe {
        let result = do_storage_get(key.as_ptr(), key.len(), buffer.as_mut_ptr(), BUFFER_SIZE);

        match result {
            -1 => Ok(None), // Not found
            len if len >= 0 => {
                let len = len as usize;
                let value = String::from_utf8(buffer[..len].to_vec())
                    .map_err(|e| format!("Failed to decode UTF-8: {}", e))?;
                Ok(Some(value))
            }
            _ => Err(format!(
                "Unexpected return value from do_storage_get: {}",
                result
            )),
        }
    }
}

/// Set a value in Durable Object storage
/// The operation is buffered and will be executed after the request completes
fn storage_set(key: &str, value: &str) -> Result<(), String> {
    unsafe {
        let result = do_storage_set(key.as_ptr(), key.len(), value.as_ptr(), value.len());

        match result {
            0 => Ok(()),
            _ => Err(format!("Failed to set value: return code {}", result)),
        }
    }
}

fn uzumibi_kernel_debug_console_log(
    vm: &mut VM,
    args: &[Rc<RObject>],
) -> Result<Rc<RObject>, mrubyedge::Error> {
    let msg_obj = &args[0];
    let msg = mrb_funcall(vm, msg_obj.clone().into(), "to_s", &[])?;
    let msg: String = msg.as_ref().try_into()?;
    unsafe {
        debug_console_log(msg.as_ptr(), msg.len());
    }
    Ok(RObject::nil().to_refcount_assigned())
}

fn uzumibi_kv_class_get(
    vm: &mut VM,
    args: &[Rc<RObject>],
) -> Result<Rc<RObject>, mrubyedge::Error> {
    let key_obj = &args[0];
    let key = mrb_funcall(vm, key_obj.clone().into(), "to_s", &[])?;
    let key: String = key.as_ref().try_into()?;

    match storage_get(&key) {
        Ok(Some(value)) => {
            let value = RObject::string(value);
            Ok(value.to_refcount_assigned())
        }
        Ok(None) => Ok(RObject::nil().to_refcount_assigned()),
        Err(e) => Err(mrubyedge::Error::RuntimeError(format!(
            "Failed to access storage value: {}",
            e
        ))),
    }
}

fn uzumibi_kv_class_set(
    vm: &mut VM,
    args: &[Rc<RObject>],
) -> Result<Rc<RObject>, mrubyedge::Error> {
    let key_obj = &args[0];
    let key = mrb_funcall(vm, key_obj.clone().into(), "to_s", &[])?;
    let key: String = key.as_ref().try_into()?;

    let value_obj = &args[1];
    let value = mrb_funcall(vm, value_obj.clone().into(), "to_s", &[])?;
    let value: String = value.as_ref().try_into()?;

    storage_set(&key, &value).map_err(|e| {
        mrubyedge::Error::RuntimeError(format!("Failed to set storage value: {}", e))
    })?;

    Ok(RObject::boolean(true).to_refcount_assigned())
}

fn init_vm() -> Result<VM, mrubyedge::Error> {
    let mut rite = rite::load(MRB)
        .map_err(|e| mrubyedge::Error::RuntimeError(format!("Failed to load mruby: {:?}", e)))?;
    let mut vm = VM::open(&mut rite);
    mruby_serde_json::init_json(&mut vm);
    uzumibi_gem::init::init_uzumibi(&mut vm);
    let object = vm.object_class.clone();
    mrb_define_cmethod(
        &mut vm,
        object.clone(),
        "debug_console",
        Box::new(uzumibi_kernel_debug_console_log),
    );

    let uzumibi_module = vm.get_module_by_name("Uzumibi");
    let kv_class = vm.define_class("KV", None, Some(uzumibi_module));
    mrb_define_class_cmethod(
        &mut vm,
        kv_class.clone(),
        "get",
        Box::new(uzumibi_kv_class_get),
    );
    mrb_define_class_cmethod(&mut vm, kv_class, "set", Box::new(uzumibi_kv_class_set));

    vm.run()
        .map_err(|e| mrubyedge::Error::RuntimeError(format!("Failed to init VM: {:?}", e)))?;

    Ok(vm)
}

fn assume_init_vm() -> Result<&'static mut VM, mrubyedge::Error> {
    unsafe {
        if !MRUBY_VM_LOADED {
            MRUBY_VM = MaybeUninit::new(init_vm()?);
            MRUBY_VM_LOADED = true;
        }
        Ok(MRUBY_VM.assume_init_mut())
    }
}

fn do_uzumibi_initialize_request(size: i32) -> Result<*mut u8, mrubyedge::Error> {
    let vm = assume_init_vm()?;
    let size = RObject::integer(size as i64).to_refcount_assigned();
    let app = vm
        .globals
        .get("$APP")
        .ok_or_else(|| mrubyedge::Error::RuntimeError("$APP is not defined".to_string()))?;
    let ret = mrb_funcall(vm, app.clone().into(), "initialize_request", &[size])?;
    ret.as_ref().try_into()
}

fn do_uzumibi_start_request() -> Result<*mut u8, mrubyedge::Error> {
    debug_console_log_internal("uzumibi_start_request called");
    let vm = assume_init_vm()?;
    let app = vm
        .globals
        .get("$APP")
        .ok_or_else(|| mrubyedge::Error::RuntimeError("$APP is not defined".to_string()))?;
    let ret = mrb_funcall(
        vm,
        app.clone().into(),
        "start_request_and_return_shared_memory",
        &[],
    )?;
    match &ret.as_ref().value {
        RValue::SharedMemory(sm) => Ok(sm.borrow_mut().leak()),
        _ => Err(mrubyedge::Error::RuntimeError(
            "Returned value is not SharedMemory".to_string(),
        )),
    }
}

#[unsafe(export_name = "uzumibi_initialize_request")]
unsafe extern "C" fn uzumibi_initialize_request(size: i32) -> u64 {
    match do_uzumibi_initialize_request(size) {
        Ok(ptr) => (ptr as u32) as u64,
        Err(e) => {
            let err_buf = set_error_to_buf(format!("Error in initialize_request: {}", e));
            ((err_buf as u32) as u64) << 32
        }
    }
}

#[unsafe(export_name = "uzumibi_start_request")]
unsafe extern "C" fn uzumibi_start_request() -> u64 {
    match do_uzumibi_start_request() {
        Ok(ptr) => (ptr as u32) as u64,
        Err(e) => {
            let err_buf = set_error_to_buf(format!("Error in start_request: {}", e));
            ((err_buf as u32) as u64) << 32
        }
    }
}
