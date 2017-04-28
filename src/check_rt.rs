use std::cell::UnsafeCell;

// use an unsafe cell to introduce as little overhead as possible
thread_local!(static RT: UnsafeCell<bool> = UnsafeCell::new(false));

/// Checks if a thread must be a realtime thread
/// Generally should only be used to ensure that slow path code is never executed in the fast path
/// with: `debug_assert!(!thread_is_realtime())`
pub fn thread_is_realtime() -> bool
{
    RT.with(|rt| unsafe { *(rt.get()) })
}

pub fn set_realtime()
{
    RT.with(|rt| unsafe {
                *(rt.get()) = true;
            });
}

pub fn set_non_realtime()
{
    RT.with(|rt| unsafe {
                *(rt.get()) = false;
            });
}

#[test]
fn not_rt_simple()
{
    set_non_realtime();
    assert!(!thread_is_realtime());
}

#[test]
fn rt_simple()
{
    set_realtime();
    assert!(thread_is_realtime());
}

#[test]
fn not_rt()
{
    use std::thread;
    set_non_realtime();

    let t = thread::spawn(|| {
        set_realtime();
        assert!(thread_is_realtime());
    });

    // main thread isn't realtime
    assert!(!thread_is_realtime());
    assert!(t.join().is_ok());
}
