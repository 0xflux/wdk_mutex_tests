//! Crate for testing the wdk-mutex crate found at:
//! https://github.com/0xflux/wdk-mutex

#![no_std]
extern crate alloc;

#[cfg(not(test))]
extern crate wdk_panic;

use core::ptr::null_mut;

use alloc::boxed::Box;
use test_kmutex::{KMutexTest, HEAP_MTX_PTR, PTR_TO_MANUAL_POOL};
use utils::ToU16Vec;
use wdk::{nt_success, println};
use wdk_alloc::WdkAllocator;
use wdk_mutex::grt::Grt;
use wdk_sys::{ntddk::{IoCreateDevice, IoCreateSymbolicLink, IoDeleteDevice, IoDeleteSymbolicLink, IofCompleteRequest, RtlInitUnicodeString}, DEVICE_OBJECT, DO_BUFFERED_IO, DRIVER_OBJECT, FILE_DEVICE_SECURE_OPEN, FILE_DEVICE_UNKNOWN, IO_NO_INCREMENT, IRP_MJ_CREATE, NTSTATUS, PCUNICODE_STRING, PDEVICE_OBJECT, PIRP, PUNICODE_STRING, STATUS_SUCCESS, STATUS_UNSUCCESSFUL, UNICODE_STRING};

mod utils;
mod test_kmutex;

#[global_allocator]
static GLOBAL_ALLOCATOR: WdkAllocator = WdkAllocator;

#[unsafe(export_name = "DriverEntry")]
pub unsafe extern "system" fn driver_entry(
    driver: &mut DRIVER_OBJECT,
    registry_path: PCUNICODE_STRING,
) -> NTSTATUS {
    //
    // Do basic driver initialisation
    //
    println!("[wdk-mutex-test] [i] Starting wdk-mutex-test");

    let status = unsafe { configure_driver(driver, registry_path as *mut _) };
    if !nt_success(status) {
        return status;
    }


    //
    // Run tests
    //

    if KMutexTest::test_multithread_mutex_global_static() == false {
        println!("[wdk-mutex-test] [-] Test test_multithread_mutex_global_static failed.");
        return STATUS_UNSUCCESSFUL;
    }

    if KMutexTest::test_multithread_mutex_global_static_manual_pool() == false {
        println!("[wdk-mutex-test] [-] Test test_multithread_mutex_global_static_manual_pool failed.");
        return STATUS_UNSUCCESSFUL;
    }

    if KMutexTest::test_to_owned() == false {
        println!("[wdk-mutex-test] [-] Test test_to_owned failed.");
        return STATUS_UNSUCCESSFUL;
    }

    if KMutexTest::test_to_owned_box() == false {
        println!("[wdk-mutex-test] [-] Test test_to_owned_box failed.");
        return STATUS_UNSUCCESSFUL;
    }

    if KMutexTest::test_grt_thrice().is_err() {
        println!("[wdk-mutex-test] [-] Test test_grt_thrice failed.");
        return STATUS_UNSUCCESSFUL;
    }

    println!("[wdk-mutex-test] [+] All tests passed. NTSTATUS: {}", status);
    status
}

/// Configuration of the driver
pub unsafe extern "C" fn configure_driver(
    driver: *mut DRIVER_OBJECT,
    _registry_path: PUNICODE_STRING,
) -> NTSTATUS {

    // GRT
    if let Err(e) = Grt::init() {
        println!("Error creating Grt! {:?}", e);
        return STATUS_UNSUCCESSFUL;
    }


    let mut dos_name = UNICODE_STRING::default();
    let mut nt_name = UNICODE_STRING::default();

    let dos_name_u16 = "\\??\\WdkMutexTest".to_u16_vec();
    let device_name_u16 = "\\Device\\WdkMutexTest".to_u16_vec();
    
    unsafe { RtlInitUnicodeString(&mut dos_name, dos_name_u16.as_ptr()) };
    unsafe { RtlInitUnicodeString(&mut nt_name, device_name_u16.as_ptr()) };

    (unsafe { *driver }).MajorFunction[IRP_MJ_CREATE as usize] = Some(create_close);
    (unsafe { *driver }).DriverUnload = Some(driver_exit);

    let mut device_object: PDEVICE_OBJECT = null_mut();
    let res = unsafe { IoCreateDevice(
        driver,
        0,
        &mut nt_name,
        FILE_DEVICE_UNKNOWN,
        FILE_DEVICE_SECURE_OPEN,
        0,
        &mut device_object,
    ) };
    if !nt_success(res) {
        println!("[wdk-mutex-test] [-] Unable to create device via IoCreateDevice. Failed with code: {res}.");
        return res;
    }
    

    let res = unsafe { IoCreateSymbolicLink(&mut dos_name, &mut nt_name) };
    if res != 0 {
        println!("[wdk-mutex-test] [-] Failed to create driver symbolic link. Error: {res}");
        return res;
    }

    //
    // Driver loaded and configured, now we can run tests outlined in the tests module.
    //


    (unsafe { *device_object }).Flags |= DO_BUFFERED_IO;

    STATUS_SUCCESS
}

/// Driver exit callback
extern "C" fn driver_exit(driver: *mut DRIVER_OBJECT) {

    // rm symbolic link
    let mut dos_name = UNICODE_STRING::default();
    let dos_name_u16 = "\\??\\WdkMutexTest".to_u16_vec();
    unsafe {
        RtlInitUnicodeString(&mut dos_name, dos_name_u16.as_ptr());
    }
    let _ = unsafe { IoDeleteSymbolicLink(&mut dos_name) };

    //
    // Clear up memory via RAII & Box
    //
    let p = HEAP_MTX_PTR.load(core::sync::atomic::Ordering::SeqCst);
    if !p.is_null() {
        let _ = unsafe { Box::from_raw(p) };
    }

    let p = PTR_TO_MANUAL_POOL.load(core::sync::atomic::Ordering::SeqCst);
    if !p.is_null() {
        let _ = unsafe { Box::from_raw(p) };
    }

    if let Err(e) = unsafe { Grt::destroy() } {
        println!("Error destroying Grt: {:?}", e);
    }

    // delete the device
    unsafe { IoDeleteDevice((*driver).DeviceObject);}

    println!("[wdk-mutex-test] [+] Driver unloaded.");
}

unsafe extern "C" fn create_close(_device: *mut DEVICE_OBJECT, pirp: PIRP) -> NTSTATUS {
    (unsafe { *pirp }).IoStatus.__bindgen_anon_1.Status = STATUS_SUCCESS;
    (unsafe { *pirp }).IoStatus.Information = 0;
    unsafe { IofCompleteRequest(pirp, IO_NO_INCREMENT as i8) };
    STATUS_SUCCESS
}