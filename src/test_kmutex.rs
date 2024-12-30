use core::{ffi::c_void, ptr::{self, null_mut}, sync::atomic::{AtomicPtr, Ordering}};

use alloc::{boxed::Box, vec::Vec};
use wdk::println;
use wdk_mutex::kmutex::KMutex;
use wdk_sys::{ntddk::{ExAllocatePool2, ExFreePool, KeGetCurrentIrql, KeWaitForSingleObject, ObReferenceObjectByHandle, ObfDereferenceObject, PsCreateSystemThread, ZwClose}, APC_LEVEL, CLIENT_ID, FALSE, HANDLE, OBJECT_ATTRIBUTES, POOL_FLAG_NON_PAGED, PVOID, STATUS_SUCCESS, THREAD_ALL_ACCESS, _KWAIT_REASON::Executive, _MODE::KernelMode};

pub static HEAP_MTX_PTR: AtomicPtr<KMutex<u32>> = AtomicPtr::new(null_mut());
pub static PTR_TO_MANUAL_POOL: AtomicPtr<KMutex<*mut u32>> = AtomicPtr::new(null_mut());

pub struct KMutexTest{}

impl KMutexTest {
    /// Tests the mutex by spawning three threads, and performing 500 mutable modifications to the
    /// T inside the mutex.
    /// 
    /// Test passes if the result == 1500.
    pub fn test_multithread_mutex_global_static() -> bool {
        
        //
        // Prepare global static for access in multiple threads.
        //
    
        let heap_mtx = Box::new(KMutex::new(0u32).unwrap());
        let heap_mtx_ptr = Box::into_raw(heap_mtx);
        HEAP_MTX_PTR.store(heap_mtx_ptr, Ordering::SeqCst);

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
                    Some(KMutexTest::callback_test_multithread_mutex_global_static), 
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
        
        let p = HEAP_MTX_PTR.load(Ordering::SeqCst);
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
            let p = HEAP_MTX_PTR.load(Ordering::SeqCst);
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
        let my_mutex: *mut KMutex<*mut u32> = Box::into_raw(Box::new(KMutex::new(my_pool_allocation).unwrap()));

        PTR_TO_MANUAL_POOL.store(my_mutex, Ordering::SeqCst);

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
                    Some(KMutexTest::callback_test_multithread_mutex_global_static_manual_pool), 
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
        
        let p: *mut KMutex<*mut u32> = PTR_TO_MANUAL_POOL.load(Ordering::SeqCst);
        if !p.is_null() {
            let k: &KMutex<*mut u32> = unsafe { &*p };

            let x = k.lock().unwrap();
            let y = unsafe { **x };
            
            let b = unsafe { Box::from_raw(p) };
            PTR_TO_MANUAL_POOL.store(null_mut(), Ordering::SeqCst);
            unsafe { ExFreePool(*b.lock().unwrap() as *mut _) };

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
            let p = PTR_TO_MANUAL_POOL.load(Ordering::SeqCst);
            if !p.is_null() {
                let p = unsafe { &*p };
                let mut lock = p.lock().unwrap();
                **lock += 1;

                // below left in for examples
                // let val = **lock;
                // println!("Value after change: {:?}", val);
            }
        }
    }


    pub fn test_to_owned() -> bool {
        
        // testing to_owned
        let m = KMutex::new(0u8).unwrap();
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
        let m = KMutex::new(0u8).unwrap();
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

}