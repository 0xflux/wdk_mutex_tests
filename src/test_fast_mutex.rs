use core::{ffi::c_void, ptr::{self, null_mut}, sync::atomic::{AtomicPtr, Ordering}};

use alloc::{boxed::Box, vec::Vec};
use wdk::{nt_success, println};
use wdk_mutex::{fast_mutex::FastMutex, grt::Grt
};
use wdk_sys::{ntddk::{ExAllocatePool2, ExFreePool, KeGetCurrentIrql, KeWaitForSingleObject, ObReferenceObjectByHandle, ObfDereferenceObject, PsCreateSystemThread, ZwClose}, APC_LEVEL, CLIENT_ID, FALSE, HANDLE, OBJECT_ATTRIBUTES, POOL_FLAG_NON_PAGED, PVOID, STATUS_SUCCESS, THREAD_ALL_ACCESS, _KWAIT_REASON::Executive, _MODE::KernelMode};

pub static HEAP_FMTX_PTR: AtomicPtr<FastMutex<u32>> = AtomicPtr::new(null_mut());
pub static PTR_TO_MANUAL_POOL_FM: AtomicPtr<FastMutex<*mut u32>> = AtomicPtr::new(null_mut());

pub struct FastMutexTest{}

impl FastMutexTest {
    /// Tests the mutex by spawning three threads, and performing 500 mutable modifications to the
    /// T inside the mutex.
    /// 
    /// Test passes if the result == 1500.
    pub fn test_multithread_mutex_global_static() -> bool {
        
        //
        // Prepare global static for access in multiple threads.
        //
    
        let heap_mtx = Box::new(FastMutex::new(0u32).unwrap());
        let heap_mtx_ptr = Box::into_raw(heap_mtx);
        HEAP_FMTX_PTR.store(heap_mtx_ptr, Ordering::SeqCst);

        let mut th = Vec::new();
    
        //
        // spawn 3 threads to test
        //
        for _ in 0..3 {
            let mut thread_handle: HANDLE = null_mut();
    
            let res = unsafe {
                PsCreateSystemThread(
                    &mut thread_handle, 
                    0, 
                    null_mut::<OBJECT_ATTRIBUTES>(), 
                    null_mut(),
                    null_mut::<CLIENT_ID>(), 
                    Some(FastMutexTest::callback_test_multithread_mutex_global_static), 
                    null_mut(),
                )
            };

            if res == STATUS_SUCCESS {
                th.push(thread_handle);
            }
        }


        //
        // Join the thread handles
        //

        for thread_handle in th {
            println!("Thread handle: {:p}, IRQL: {}", thread_handle, unsafe{KeGetCurrentIrql()});

            if !thread_handle.is_null() && unsafe{KeGetCurrentIrql()} <= APC_LEVEL as u8 {
                let mut thread_obj: PVOID = null_mut();
                let ref_status = unsafe {
                    ObReferenceObjectByHandle(
                        thread_handle,
                        THREAD_ALL_ACCESS,
                        null_mut(),
                        KernelMode as i8,
                        &mut thread_obj,
                        null_mut(),
                    )
                };
                unsafe { let _ = ZwClose(thread_handle); };

                if ref_status == STATUS_SUCCESS {
                    unsafe {
                        let _ = KeWaitForSingleObject(
                            thread_obj,
                            Executive,
                            KernelMode as i8,
                            FALSE as u8,
                            null_mut(),
                        );
                    }
                    
                unsafe { ObfDereferenceObject(thread_obj) };
                }
                
            }
        }


        //
        // Check the result
        //
        const RESULT_VAL: u32 = 1500;
        
        let p = HEAP_FMTX_PTR.load(Ordering::SeqCst);
        if !p.is_null() {
            let p = unsafe { &*p };
            if *p.lock().unwrap() != RESULT_VAL {
                return false;
            }
        } else {
            println!("[wdk-mutex-tests] Heap pointer was null.");
            return false;
        }

        true

    }
    
    /// Callback function for operating on a global static AtomicPtr
    unsafe extern "C" fn callback_test_multithread_mutex_global_static(_: *mut c_void) {
        for _ in 0..500 {
            let p = HEAP_FMTX_PTR.load(Ordering::SeqCst);
            if !p.is_null() {
                let p = unsafe { &*p };
                let mut lock = p.lock().unwrap();
                *lock += 1;
            }
        }
    }


    pub fn test_multithread_mutex_global_static_manual_pool() -> bool {
        //
        // Prepare global static for access in multiple threads.
        //
        
        let my_pool_allocation: *mut u32 = unsafe {
            ExAllocatePool2(POOL_FLAG_NON_PAGED, size_of::<u32>() as u64, u32::from_be_bytes(*b"kmtx"))
        } as *mut u32;
        unsafe {ptr::write(my_pool_allocation, 0u32)};
        let my_mutex: *mut FastMutex<*mut u32> = Box::into_raw(Box::new(FastMutex::new(my_pool_allocation).unwrap()));

        PTR_TO_MANUAL_POOL_FM.store(my_mutex, Ordering::SeqCst);

        let mut th = Vec::new();
    
        //
        // spawn 3 threads to test
        //
        for _ in 0..3 {
            let mut thread_handle: HANDLE = null_mut();
    
            let res = unsafe {
                PsCreateSystemThread(
                    &mut thread_handle, 
                    0, 
                    null_mut::<OBJECT_ATTRIBUTES>(), 
                    null_mut(),
                    null_mut::<CLIENT_ID>(), 
                    Some(FastMutexTest::callback_test_multithread_mutex_global_static_manual_pool), 
                    null_mut(),
                )
            };

            if res == STATUS_SUCCESS {
                th.push(thread_handle);
            }
        }


        //
        // Join the thread handles
        //

        for thread_handle in th {
            if !thread_handle.is_null() && unsafe{KeGetCurrentIrql()} <= APC_LEVEL as u8 {
                let mut thread_obj: PVOID = null_mut();
                let ref_status = unsafe {
                    ObReferenceObjectByHandle(
                        thread_handle,
                        THREAD_ALL_ACCESS,
                        null_mut(),
                        KernelMode as i8,
                        &mut thread_obj,
                        null_mut(),
                    )
                };
                unsafe { let _ = ZwClose(thread_handle); };

                if ref_status == STATUS_SUCCESS {
                    unsafe {
                        let _ = KeWaitForSingleObject(
                            thread_obj,
                            Executive,
                            KernelMode as i8,
                            FALSE as u8,
                            null_mut(),
                        );
                    }
                    
                unsafe { ObfDereferenceObject(thread_obj) };
                }
                
            }
        }


        //
        // Check the result
        //
        const RESULT_VAL: u32 = 1500;
        
        let p: *mut FastMutex<*mut u32> = PTR_TO_MANUAL_POOL_FM.load(Ordering::SeqCst);
        if !p.is_null() {
            let k: &FastMutex<*mut u32> = unsafe { &*p };

            let x = k.lock().unwrap();
            let y = unsafe { **x };
            
            if y != RESULT_VAL {
                return false;
            }
        } else {
            println!("[wdk-mutex-tests] PTR_TO_MANUAL_POOL was null.");
            return false;
        }

        true
    }
    
    unsafe extern "C" fn callback_test_multithread_mutex_global_static_manual_pool(_: *mut c_void) {
        for _ in 0..500 {
            let p = PTR_TO_MANUAL_POOL_FM.load(Ordering::SeqCst);
            if !p.is_null() {
                let p = unsafe { &*p };
                let mut lock = p.lock().unwrap();
                unsafe { **lock += 1 };

                // below left in for examples
                // let val = **lock;
                // println!("Value after change: {:?}", val);
            }
        }
    }


    pub fn test_to_owned() -> bool {
        
        // testing to_owned
        let m = FastMutex::new(0u8).unwrap();
        {
            let mut lock = m.lock().unwrap();
            *lock += 1;
        }

        let x = unsafe { m.to_owned() };

        if x == 1 {
            true
        } else {
            false
        }
    }

    pub fn test_to_owned_box() -> bool {
        
        // testing to_owned
        let m = FastMutex::new(0u8).unwrap();
        {
            let mut lock = m.lock().unwrap();
            *lock += 1;
        }

        let x = unsafe { m.to_owned_box() };

        if *x == 1 {
            true
        } else {
            false
        }
    }

    pub fn test_grt_thrice() -> Result<(), ()> {
        
        test_grt()?;
        
        Ok(())
    }

}

pub fn test_grt() -> Result<(), ()>{
    let mut th = Vec::new();

    let _ = Grt::register_fast_mutex("my_test_mutex", 0u32);
    
    for _ in 0..3 {
        let mut thread_handle: HANDLE = null_mut();

        let status = unsafe {
            PsCreateSystemThread(
                &mut thread_handle, 
                0, 
                null_mut::<OBJECT_ATTRIBUTES>(), 
                null_mut(),
                null_mut::<CLIENT_ID>(), 
                Some(callback_fn_grt), 
                null_mut(),
            )
        };

        if nt_success(status) {
            th.push(thread_handle);
        }
    }

    test_grt2()?;

    //
    // Join the thread handles
    //

    for thread_handle in th {

        if !thread_handle.is_null() && unsafe{KeGetCurrentIrql()} <= APC_LEVEL as u8 {
            let mut thread_obj: PVOID = null_mut();
            let ref_status = unsafe {
                ObReferenceObjectByHandle(
                    thread_handle,
                    THREAD_ALL_ACCESS,
                    null_mut(),
                    KernelMode as i8,
                    &mut thread_obj,
                    null_mut(),
                )
            };
            unsafe { let _ = ZwClose(thread_handle); };

            if ref_status == STATUS_SUCCESS {
                unsafe {
                    let _ = KeWaitForSingleObject(
                        thread_obj,
                        Executive,
                        KernelMode as i8,
                        FALSE as u8,
                        null_mut(),
                    );
                }
                
            unsafe { ObfDereferenceObject(thread_obj) };
            }
        }
    }

    let my_mut = Grt::get_fast_mutex::<u32>("my_test_mutex");
    if let Err(e) = my_mut {
        println!("Error in callback: {:?}", e);
        return Err(());
    }

    let lock = my_mut.unwrap().lock().unwrap();
    if *lock != 300 {
        return Err(())
    }

    Ok(())

}


unsafe extern "C" fn callback_fn_grt(_: *mut c_void) {
    for _ in 0..100 {
        let my_mut = Grt::get_fast_mutex::<u32>("my_test_mutex");
        if let Err(e) = my_mut {
            println!("Error in callback: {:?}", e);
            return;
        }

        let mut lock = my_mut.unwrap().lock().unwrap();
        *lock += 1;
    }
}



pub fn test_grt2() -> Result<(), ()>{
    let mut th = Vec::new();

    if let Err(e) = Grt::register_fast_mutex("my_test_mutex2", 0u32) {
        println!("ERROR registering mutex: {:?}", e);
        return Err(());
    };

    test_grt3()?;
    
    for _ in 0..3 {
        let mut thread_handle: HANDLE = null_mut();

        let status = unsafe {
            PsCreateSystemThread(
                &mut thread_handle, 
                0, 
                null_mut::<OBJECT_ATTRIBUTES>(), 
                null_mut(),
                null_mut::<CLIENT_ID>(), 
                Some(callback_fn_grt_2), 
                null_mut(),
            )
        };

        if nt_success(status) {
            th.push(thread_handle);
        }
    }

    //
    // Join the thread handles
    //

    for thread_handle in th {

        if !thread_handle.is_null() && unsafe{KeGetCurrentIrql()} <= APC_LEVEL as u8 {
            let mut thread_obj: PVOID = null_mut();
            let ref_status = unsafe {
                ObReferenceObjectByHandle(
                    thread_handle,
                    THREAD_ALL_ACCESS,
                    null_mut(),
                    KernelMode as i8,
                    &mut thread_obj,
                    null_mut(),
                )
            };
            unsafe { let _ = ZwClose(thread_handle); };

            if ref_status == STATUS_SUCCESS {
                unsafe {
                    let _ = KeWaitForSingleObject(
                        thread_obj,
                        Executive,
                        KernelMode as i8,
                        FALSE as u8,
                        null_mut(),
                    );
                }
                
            unsafe { ObfDereferenceObject(thread_obj) };
            }
        }
    }

    let my_mut = Grt::get_fast_mutex::<u32>("my_test_mutex2");
    if let Err(e) = my_mut {
        println!("Error in callback: {:?}", e);
        return Err(());
    }

    let lock = my_mut.unwrap().lock().unwrap();
    if *lock != 300 {
        return Err(())
    }
    
    Ok(())
}

unsafe extern "C" fn callback_fn_grt_2(_: *mut c_void) {
    for _ in 0..100 {
        let my_mut = Grt::get_fast_mutex::<u32>("my_test_mutex2");
        if let Err(e) = my_mut {
            println!("Error in callback: {:?}", e);
            return;
        }

        let mut lock = my_mut.unwrap().lock().unwrap();
        *lock += 1;
    }
}



pub fn test_grt3() -> Result<(), ()> {
    let mut th = Vec::new();

    if let Err(e) = Grt::register_fast_mutex("my_test_mutex3", 0u32) {
        println!("ERROR registering mutex: {:?}", e);
        return Err(());
    };

    for _ in 0..3 {
        let mut thread_handle: HANDLE = null_mut();

        let status = unsafe {
            PsCreateSystemThread(
                &mut thread_handle, 
                0, 
                null_mut::<OBJECT_ATTRIBUTES>(), 
                null_mut(),
                null_mut::<CLIENT_ID>(), 
                Some(callback_fn_grt_3), 
                null_mut(),
            )
        };

        if nt_success(status) {
            th.push(thread_handle);
        }
    }

    //
    // Join the thread handles
    //

    for thread_handle in th {
        if !thread_handle.is_null() && unsafe{KeGetCurrentIrql()} <= APC_LEVEL as u8 {
            let mut thread_obj: PVOID = null_mut();
            let ref_status = unsafe {
                ObReferenceObjectByHandle(
                    thread_handle,
                    THREAD_ALL_ACCESS,
                    null_mut(),
                    KernelMode as i8,
                    &mut thread_obj,
                    null_mut(),
                )
            };
            unsafe { let _ = ZwClose(thread_handle); };

            if ref_status == STATUS_SUCCESS {
                unsafe {
                    let _ = KeWaitForSingleObject(
                        thread_obj,
                        Executive,
                        KernelMode as i8,
                        FALSE as u8,
                        null_mut(),
                    );
                }
                
            unsafe { ObfDereferenceObject(thread_obj) };
            }
        }
    }

    let my_mut = Grt::get_fast_mutex::<u32>("my_test_mutex3");
    if let Err(e) = my_mut {
        println!("Error in callback: {:?}", e);
        return Err(());
    }

    let lock = my_mut.unwrap().lock().unwrap();
    if *lock != 300 {
        return Err(())
    }

    Ok(())
}

unsafe extern "C" fn callback_fn_grt_3(_: *mut c_void) {
    for _ in 0..100 {
        let my_mut = Grt::get_fast_mutex::<u32>("my_test_mutex3");
        if let Err(e) = my_mut {
            println!("Error in callback: {:?}", e);
            return;
        }

        let mut lock = my_mut.unwrap().lock().unwrap();
        *lock += 1;
    }
}